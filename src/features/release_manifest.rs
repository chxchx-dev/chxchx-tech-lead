//! The release manifest and its signature (ADR 0017, H6).
//!
//! `studio-manifest.txt` lists each package of a release with its SHA-256. CI signs that file with
//! an ed25519 key whose private half lives only in a CI secret; the program carries the public half
//! (`packaging/release-public-key.txt`). An update is applied only if the signature checks out, the
//! package's SHA-256 matches the manifest and the version is newer. With no public key built in
//! nothing is ever applied: the person is sent to the release page as before.

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

/// The key this build trusts, or `None` when none was configured.
pub fn built_in_key() -> Option<[u8; 32]> {
    parse_key(include_str!("../../packaging/release-public-key.txt"))
}

/// A key written as 64 hexadecimal digits; lines starting with `#` are comments.
pub fn parse_key(text: &str) -> Option<[u8; 32]> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))?;
    from_hex::<32>(line)
}

pub fn from_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != N * 2 || !text.is_ascii() {
        return None;
    }
    let mut out = [0u8; N];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(out)
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    to_hex(&Sha256::digest(bytes))
}

/// Checks that `signature_hex` (128 hex digits) is `key`'s signature of `manifest`.
pub fn verify(manifest: &[u8], signature_hex: &str, key: &[u8; 32]) -> Result<(), String> {
    let signature = from_hex::<64>(signature_hex.trim())
        .ok_or("La firma del lanzamiento no tiene el formato esperado.")?;
    let key =
        VerifyingKey::from_bytes(key).map_err(|_| "La clave pública incrustada no es válida.")?;
    key.verify_strict(manifest, &Signature::from_bytes(&signature))
        .map_err(|_| "La firma del lanzamiento NO es válida: no se aplica nada.".to_owned())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub channel: String,
    pub version: String,
    pub commit: String,
    /// `(key, file name, sha256)`
    pub assets: Vec<(String, String, String)>,
}

pub fn parse(text: &str) -> Result<Manifest, String> {
    let mut values = std::collections::HashMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    let get = |key: &str| {
        values
            .get(key)
            .cloned()
            .ok_or_else(|| format!("Al manifiesto le falta `{key}`."))
    };
    let mut assets = Vec::new();
    for (key, name) in &values {
        let Some(id) = key.strip_prefix("asset.") else {
            continue;
        };
        let digest = get(&format!("sha256.{id}"))?;
        // Names come from the network: only a plain file name is accepted.
        if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
            return Err(format!(
                "Nombre de archivo no válido en el manifiesto: {name}"
            ));
        }
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("SHA-256 no válido para {id}."));
        }
        assets.push((id.to_owned(), name.clone(), digest.to_ascii_lowercase()));
    }
    assets.sort();
    Ok(Manifest {
        channel: get("channel")?,
        version: get("version")?,
        commit: get("commit")?,
        assets,
    })
}

impl Manifest {
    /// The package that replaces this machine's installation, if there is one this program can apply.
    pub fn linux_package(&self) -> Option<&(String, String, String)> {
        ["linux-ubuntu-x86_64", "linux-fedora-x86_64"]
            .iter()
            .find_map(|wanted| self.assets.iter().find(|(key, _, _)| key == wanted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn signed(text: &str, seed: u8) -> (String, [u8; 32]) {
        use ed25519_dalek::Signer;
        let key = SigningKey::from_bytes(&[seed; 32]);
        (
            to_hex(&key.sign(text.as_bytes()).to_bytes()),
            key.verifying_key().to_bytes(),
        )
    }

    const SAMPLE: &str = "schema_version=1\nchannel=stable\ntag=v0.7.0\ncommit=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nversion=0.7.0\nasset.linux-ubuntu-x86_64=chxchx-studio-ubuntu-x86_64.tar.gz\nsha256.linux-ubuntu-x86_64=0000000000000000000000000000000000000000000000000000000000000000\n";

    #[test]
    fn a_good_signature_is_accepted_and_any_change_is_not() {
        let (signature, key) = signed(SAMPLE, 7);
        assert!(verify(SAMPLE.as_bytes(), &signature, &key).is_ok());
        let tampered = SAMPLE.replace("0.7.0", "9.9.9");
        assert!(verify(tampered.as_bytes(), &signature, &key).is_err());
        let (_, other) = signed(SAMPLE, 8);
        assert!(verify(SAMPLE.as_bytes(), &signature, &other).is_err());
        assert!(verify(SAMPLE.as_bytes(), "not hex", &key).is_err());
    }

    #[test]
    fn the_manifest_is_read_and_a_hostile_file_name_is_refused() {
        let manifest = parse(SAMPLE).unwrap();
        assert_eq!(manifest.version, "0.7.0");
        assert_eq!(
            manifest.linux_package().unwrap().1,
            "chxchx-studio-ubuntu-x86_64.tar.gz"
        );
        let hostile = SAMPLE.replace("chxchx-studio-ubuntu-x86_64.tar.gz", "../../etc/passwd");
        assert!(parse(&hostile).is_err());
        assert!(parse("version=1\n").is_err());
    }

    #[test]
    fn the_shipped_key_file_is_a_comment_until_someone_generates_one() {
        assert_eq!(parse_key("# nada\n"), None);
        let key = to_hex(&[3u8; 32]);
        assert_eq!(parse_key(&format!("# c\n{key}\n")), Some([3u8; 32]));
        assert_eq!(parse_key("zz"), None);
    }

    /// `packaging/sign_manifest.py` (what CI runs) signed b"hola" with the seed `05` x 32.
    #[test]
    fn what_the_ci_script_signs_this_program_accepts() {
        let key =
            from_hex::<32>("6e7a1cdd29b0b78fd13af4c5598feff4ef2a97166e3ca6f2e4fbfccd80505bf1")
                .unwrap();
        let signature = "f2d20d2d8e923b3e3e0680927042b1b2ce08e6a347afcb72426846828a40653c9137cfd6ca65675ea526c91891a7de1c6b56a9726c5fc3a6a5541b678a207600";
        assert!(verify(b"hola", signature, &key).is_ok());
        assert!(verify(b"hola!", signature, &key).is_err());
    }

    #[test]
    fn sha256_matches_a_known_value() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
