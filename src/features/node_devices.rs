//! Devices linked to the node: pairing codes, device tokens, revocation (ADR 0017, H3).
//!
//! A device is linked on the PC: the person asks for a one-use code, types it on the new device
//! within five minutes, and receives a token that is shown once. Only the SHA-256 of that token
//! (and of a pending code) is kept, so a copy of `devices.json` cannot be used to enter. Tokens
//! are 256 random bits, so a hash lookup is enough; a pairing code is shorter, so guessing it is
//! limited: five wrong tries close pairing for ten minutes and cancel every pending code.

use crate::features::node_state::write_private;
use chxchx_protocol::{
    Device, DeviceId, ErrorCode, JobError, PAIRING_TTL_MS, PairingOffer, Role,
    normalize_pairing_code, validate_device_name,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // no I, O, 0, 1
const CODE_LENGTH: usize = 10;
const MAX_DEVICES: usize = 20;
const MAX_PENDING_CODES: usize = 3;
/// Wrong codes tolerated inside `FAILURE_WINDOW_MS` before pairing closes.
const MAX_FAILURES: usize = 5;
const FAILURE_WINDOW_MS: u64 = 10 * 60 * 1000;

/// Fills the buffer with cryptographically secure random bytes.
pub type Random<'a> = &'a mut dyn FnMut(&mut [u8]);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Stored {
    id: DeviceId,
    name: String,
    role: Role,
    created_ms: u64,
    /// Lower-case hex SHA-256 of the token.
    token_hash: String,
}

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    devices: Vec<Stored>,
}

struct Pending {
    code_hash: String,
    role: Role,
    expires_ms: u64,
}

pub struct Devices {
    path: PathBuf,
    stored: Vec<Stored>,
    pending: Vec<Pending>,
    failures: VecDeque<u64>,
    seen: BTreeMap<DeviceId, u64>,
}

impl Devices {
    pub fn open(dir: &Path) -> Result<Self, String> {
        let path = dir.join("devices.json");
        let stored = match fs::read(&path) {
            Ok(bytes) => {
                serde_json::from_slice::<File>(&bytes)
                    .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?
                    .devices
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(format!("No se pudo leer {}: {error}", path.display())),
        };
        Ok(Self {
            path,
            stored,
            pending: Vec::new(),
            failures: VecDeque::new(),
            seen: BTreeMap::new(),
        })
    }

    /// A new one-use code for a device that will get `role`.
    pub fn offer(
        &mut self,
        role: Role,
        now_ms: u64,
        random: Random<'_>,
    ) -> Result<PairingOffer, JobError> {
        self.pending.retain(|pending| pending.expires_ms > now_ms);
        if self.pending.len() >= MAX_PENDING_CODES {
            return Err(JobError::new(
                ErrorCode::Conflict,
                "Ya hay códigos de vinculación sin usar; espera a que caduquen (5 min)",
            ));
        }
        if self.stored.len() >= MAX_DEVICES {
            return Err(JobError::new(
                ErrorCode::Conflict,
                "Hay demasiados dispositivos vinculados; revoca alguno primero",
            ));
        }
        let mut bytes = [0u8; CODE_LENGTH];
        random(&mut bytes);
        let letters: String = bytes
            .iter()
            .map(|byte| CODE_ALPHABET[usize::from(*byte) % CODE_ALPHABET.len()] as char)
            .collect();
        let expires_ms = now_ms + PAIRING_TTL_MS;
        self.pending.push(Pending {
            code_hash: sha256_hex(&letters),
            role,
            expires_ms,
        });
        Ok(PairingOffer {
            code: format!("{}-{}", &letters[..5], &letters[5..]),
            role,
            expires_ms,
        })
    }

    /// Links a device with the code it was shown. The token comes back once and is not kept.
    pub fn pair(
        &mut self,
        code: &str,
        name: &str,
        now_ms: u64,
        random: Random<'_>,
    ) -> Result<(Device, String), JobError> {
        validate_device_name(name)?;
        self.failures
            .retain(|at| now_ms.saturating_sub(*at) < FAILURE_WINDOW_MS);
        if self.failures.len() >= MAX_FAILURES {
            return Err(JobError::new(
                ErrorCode::TooManyRequests,
                "Demasiados intentos con códigos incorrectos; la vinculación está cerrada unos minutos",
            ));
        }
        self.pending.retain(|pending| pending.expires_ms > now_ms);
        let wanted = sha256_hex(&normalize_pairing_code(code));
        let position = self
            .pending
            .iter()
            .position(|pending| same(pending.code_hash.as_bytes(), wanted.as_bytes()));
        let Some(position) = position else {
            self.failures.push_back(now_ms);
            if self.failures.len() >= MAX_FAILURES {
                // Someone is guessing: no code that was waiting stays valid.
                self.pending.clear();
            }
            return Err(JobError::new(
                ErrorCode::Unauthorized,
                "Código de vinculación incorrecto o caducado",
            ));
        };
        let pending = self.pending.remove(position);
        let mut id_bytes = [0u8; 8];
        random(&mut id_bytes);
        let mut token_bytes = [0u8; 32];
        random(&mut token_bytes);
        let token = hex(&token_bytes);
        let stored = Stored {
            id: DeviceId::parse(format!("dev-{}", hex(&id_bytes)))
                .expect("generated ids are valid"),
            name: name.trim().to_owned(),
            role: pending.role,
            created_ms: now_ms,
            token_hash: sha256_hex(&token),
        };
        let device = shown(&stored, None);
        self.stored.push(stored);
        if let Err(message) = self.save() {
            self.stored.pop(); // not linked if it could not be written down
            return Err(JobError::new(ErrorCode::Internal, message));
        }
        Ok((device, token))
    }

    /// The device that owns `token`, if it is linked and not revoked.
    pub fn authenticate(&mut self, token: &str, now_ms: u64) -> Option<Device> {
        let wanted = sha256_hex(token);
        let stored = self
            .stored
            .iter()
            .find(|stored| same(stored.token_hash.as_bytes(), wanted.as_bytes()))?;
        let id = stored.id.clone();
        let device = shown(stored, Some(now_ms));
        self.seen.insert(id, now_ms);
        Some(device)
    }

    pub fn is_linked(&self, id: &DeviceId) -> bool {
        self.stored.iter().any(|stored| stored.id == *id)
    }

    pub fn list(&self) -> Vec<Device> {
        self.stored
            .iter()
            .map(|stored| shown(stored, self.seen.get(&stored.id).copied()))
            .collect()
    }

    /// Unlinks a device at once: its token stops working on the next request.
    pub fn revoke(&mut self, id: &DeviceId) -> Result<Device, JobError> {
        let position = self
            .stored
            .iter()
            .position(|stored| stored.id == *id)
            .ok_or_else(|| {
                JobError::new(ErrorCode::NotFound, format!("No hay un dispositivo {id}"))
            })?;
        let removed = self.stored.remove(position);
        if let Err(message) = self.save() {
            self.stored.insert(position, removed);
            return Err(JobError::new(ErrorCode::Internal, message));
        }
        self.seen.remove(id);
        Ok(shown(&removed, None))
    }

    fn save(&self) -> Result<(), String> {
        let file = File {
            version: 1,
            devices: self.stored.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&file).map_err(|error| error.to_string())?;
        write_private(&self.path, &bytes)
    }
}

fn shown(stored: &Stored, last_seen_ms: Option<u64>) -> Device {
    Device {
        id: stored.id.clone(),
        name: stored.name.clone(),
        role: stored.role,
        created_ms: stored.created_ms,
        last_seen_ms,
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256_hex(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}

/// Compares without stopping at the first difference.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Predictable "randomness": every call continues a counter, so values never repeat.
    fn counter() -> impl FnMut(&mut [u8]) {
        let mut next = 1u8;
        move |buffer: &mut [u8]| {
            for byte in buffer {
                *byte = next;
                next = next.wrapping_add(7);
            }
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("chxchx-devices-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn link(
        devices: &mut Devices,
        role: Role,
        now: u64,
        rng: &mut impl FnMut(&mut [u8]),
    ) -> (Device, String) {
        let offer = devices.offer(role, now, rng).unwrap();
        devices.pair(&offer.code, "Pixel", now, rng).unwrap()
    }

    #[test]
    fn a_device_is_linked_once_with_a_code_and_then_known_by_its_token() {
        let dir = scratch("link");
        let mut rng = counter();
        let mut devices = Devices::open(&dir).unwrap();
        let offer = devices.offer(Role::Operator, 1_000, &mut rng).unwrap();
        assert_eq!(offer.code.len(), 11, "XXXXX-XXXXX");
        assert_eq!(offer.expires_ms, 1_000 + PAIRING_TTL_MS);

        let (device, token) = devices
            .pair(
                &offer.code.to_lowercase(),
                "  Pixel de Ana ",
                2_000,
                &mut rng,
            )
            .unwrap();
        assert_eq!(
            (device.name.as_str(), device.role),
            ("Pixel de Ana", Role::Operator)
        );
        assert_eq!(token.len(), 64);
        assert_eq!(devices.authenticate(&token, 3_000).unwrap().id, device.id);
        assert!(
            devices
                .authenticate("0".repeat(64).as_str(), 3_000)
                .is_none()
        );

        let again = devices
            .pair(&offer.code, "Otro", 2_500, &mut rng)
            .unwrap_err();
        assert_eq!(again.code, ErrorCode::Unauthorized, "a code works once");
    }

    #[test]
    fn only_hashes_reach_the_disk_and_devices_survive_a_restart() {
        let dir = scratch("disk");
        let mut rng = counter();
        let mut devices = Devices::open(&dir).unwrap();
        let (device, token) = link(&mut devices, Role::Reader, 1, &mut rng);
        let text = fs::read_to_string(dir.join("devices.json")).unwrap();
        assert!(!text.contains(&token), "the token itself is never written");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.join("devices.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let mut reopened = Devices::open(&dir).unwrap();
        assert_eq!(reopened.authenticate(&token, 9).unwrap().id, device.id);
        assert_eq!(reopened.list().len(), 1);
    }

    #[test]
    fn a_code_expires_and_pending_codes_are_limited() {
        let mut devices = Devices::open(&scratch("expiry")).unwrap();
        let mut rng = counter();
        let offer = devices.offer(Role::Operator, 0, &mut rng).unwrap();
        let late = devices
            .pair(&offer.code, "Tarde", PAIRING_TTL_MS + 1, &mut rng)
            .unwrap_err();
        assert_eq!(late.code, ErrorCode::Unauthorized);

        let mut devices = Devices::open(&scratch("limit")).unwrap();
        for _ in 0..3 {
            devices.offer(Role::Reader, 0, &mut rng).unwrap();
        }
        assert_eq!(
            devices.offer(Role::Reader, 0, &mut rng).unwrap_err().code,
            ErrorCode::Conflict
        );
        assert!(
            devices
                .offer(Role::Reader, PAIRING_TTL_MS + 1, &mut rng)
                .is_ok(),
            "expired codes make room"
        );
    }

    #[test]
    fn guessing_codes_closes_pairing_and_cancels_the_ones_waiting() {
        let mut devices = Devices::open(&scratch("guess")).unwrap();
        let mut rng = counter();
        let offer = devices.offer(Role::Admin, 0, &mut rng).unwrap();
        for attempt in 0..5 {
            let error = devices
                .pair("AAAAA-AAAAA", "Intruso", attempt, &mut rng)
                .unwrap_err();
            assert_eq!(error.code, ErrorCode::Unauthorized);
        }
        let closed = devices
            .pair(&offer.code, "Legítimo", 10, &mut rng)
            .unwrap_err();
        assert_eq!(
            closed.code,
            ErrorCode::TooManyRequests,
            "even the right code, now"
        );
        let after = FAILURE_WINDOW_MS + 100;
        let still = devices
            .pair(&offer.code, "Legítimo", after, &mut rng)
            .unwrap_err();
        assert_eq!(
            still.code,
            ErrorCode::Unauthorized,
            "the code was cancelled when guessing began"
        );
        let fresh = devices.offer(Role::Admin, after, &mut rng).unwrap();
        assert!(
            devices
                .pair(&fresh.code, "Legítimo", after, &mut rng)
                .is_ok()
        );
    }

    #[test]
    fn revoking_a_device_stops_its_token_at_once_and_forever() {
        let dir = scratch("revoke");
        let mut rng = counter();
        let mut devices = Devices::open(&dir).unwrap();
        let (device, token) = link(&mut devices, Role::Operator, 1, &mut rng);
        assert!(devices.is_linked(&device.id));
        devices.revoke(&device.id).unwrap();
        assert!(devices.authenticate(&token, 2).is_none());
        assert!(!devices.is_linked(&device.id));
        assert_eq!(
            devices.revoke(&device.id).unwrap_err().code,
            ErrorCode::NotFound
        );
        assert!(
            Devices::open(&dir).unwrap().list().is_empty(),
            "also after a restart"
        );
    }

    #[test]
    fn names_are_checked_and_the_number_of_devices_is_capped() {
        let mut devices = Devices::open(&scratch("names")).unwrap();
        let mut rng = counter();
        let offer = devices.offer(Role::Reader, 0, &mut rng).unwrap();
        assert_eq!(
            devices
                .pair(&offer.code, "a\nb", 0, &mut rng)
                .unwrap_err()
                .code,
            ErrorCode::InvalidRequest
        );
        assert!(
            devices.pair(&offer.code, "ok", 0, &mut rng).is_ok(),
            "a bad name did not burn the code"
        );
        for _ in 0..MAX_DEVICES - 1 {
            link(&mut devices, Role::Reader, 0, &mut rng);
        }
        assert_eq!(
            devices.offer(Role::Reader, 0, &mut rng).unwrap_err().code,
            ErrorCode::Conflict
        );
    }
}
