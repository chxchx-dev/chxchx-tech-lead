//! Transcript of a visual agent chat, built from the `chxchx.agent-event` v1 lines that
//! `chxchx-tech agent chat` prints. It owns no process and no widgets, so it is tested headless;
//! Studio renders it. Same behavior as the legacy Studio's model, which this replaces.
use serde_json::Value;

/// A tool's output is kept up to this many characters (its end, where the result is).
pub const MAX_OUTPUT: usize = 8_000;

/// The last `max` characters of `text`.
fn tail_of(text: String, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text;
    }
    let skipped: String = text.chars().skip(count - max).collect();
    format!("…{skipped}")
}

pub const SCHEMA: &str = "chxchx.agent-event";
pub const SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    User,
    Assistant,
    Reasoning,
    Tool,
    Notice,
    Error,
    Permission,
    Question,
}

/// One way to answer a permission card.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Choice {
    pub label: String,
    /// Rules this choice grants (`Bash(ls:*)`, `bash:git *`, `edit`…).
    pub allow: Vec<String>,
    pub network: bool,
    /// An access level to switch to for the retry (`full-access`).
    pub mode: Option<String>,
    /// Remembered for the rest of the chat, not only for this request.
    pub persist: bool,
    /// Codex: approve for the rest of the turn.
    pub scope: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Question {
    pub question: String,
    pub header: String,
    pub multi_select: bool,
    pub options: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub kind: Kind,
    pub text: String,
    pub tool_id: String,
    pub tool_kind: String,
    pub status: String,
    pub output: String,
    pub choices: Vec<Choice>,
    pub certain: bool,
    /// What the person answered ("" while the card is open).
    pub decision: String,
    /// A live request: the agent is paused until Studio answers this id.
    pub request_id: String,
    pub questions: Vec<Question>,
    /// Bumped when an event updates the entry in place.
    pub revision: u32,
}

impl Entry {
    fn new(kind: Kind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            tool_id: String::new(),
            tool_kind: String::new(),
            status: String::new(),
            output: String::new(),
            choices: Vec::new(),
            certain: true,
            decision: String::new(),
            request_id: String::new(),
            questions: Vec::new(),
            revision: 0,
        }
    }

    fn is_open_request(&self) -> bool {
        matches!(self.kind, Kind::Permission | Kind::Question)
            && !self.request_id.is_empty()
            && self.decision.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub used_percent: f64,
    pub window_minutes: u32,
    pub resets_at: i64,
}

/// What the session has spent and how much room is left.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: Option<f64>,
    pub context_tokens: i64,
    pub context_window: i64,
    pub primary: Option<Window>,
    pub secondary: Option<Window>,
    pub plan: String,
    /// When the limits were last read (unix seconds); 0 when never.
    pub limits_at: i64,
}

impl Usage {
    /// How full the context is, 0..=100, when the agent reported both numbers.
    pub fn context_percent(&self) -> Option<f64> {
        (self.context_tokens > 0 && self.context_window > 0)
            .then(|| (self.context_tokens as f64 / self.context_window as f64 * 100.0).min(100.0))
    }

    /// "40k de entrada · 1k de salida · US$ 0.1200", empty before the first answer.
    pub fn summary(&self) -> String {
        if self.input_tokens == 0 && self.output_tokens == 0 {
            return String::new();
        }
        let mut text = format!(
            "{} de entrada · {} de salida",
            compact_tokens(self.input_tokens),
            compact_tokens(self.output_tokens)
        );
        if let Some(cost) = self.cost_usd {
            text.push_str(&format!(" · US$ {cost:.4}"));
        }
        text
    }
}

/// 1234 -> "1k", 1500000 -> "1.5M".
pub fn compact_tokens(tokens: i64) -> String {
    let compact = |value: f64, suffix: &str| {
        let text = format!("{value:.1}");
        format!("{}{suffix}", text.strip_suffix(".0").unwrap_or(&text))
    };
    match tokens {
        t if t >= 1_000_000 => compact(t as f64 / 1_000_000.0, "M"),
        t if t >= 1_000 => compact(t as f64 / 1_000.0, "k"),
        t => t.to_string(),
    }
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub entries: Vec<Entry>,
    pub session_id: String,
    pub usage: Usage,
    /// What the running turn has used so far, by message (a new report of the same message replaces
    /// the old one): (input, output). Cleared when the turn's final usage arrives or the turn ends.
    pub live: std::collections::BTreeMap<String, (i64, i64)>,
}

fn text_of(event: &Value, key: &str) -> String {
    event
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn window_of(value: Option<&Value>) -> Option<Window> {
    let value = value?;
    let used = value.get("used_percent").and_then(Value::as_f64)?;
    (used >= 0.0).then(|| Window {
        used_percent: used,
        window_minutes: value
            .get("window_minutes")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        resets_at: value.get("resets_at").and_then(Value::as_i64).unwrap_or(0),
    })
}

fn choices_of(value: Option<&Value>) -> Vec<Choice> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|choice| Choice {
            label: text_of(choice, "label"),
            allow: choice
                .get("allow")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|rule| rule.as_str().map(str::to_owned))
                .collect(),
            network: choice
                .get("network")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            mode: choice
                .get("mode")
                .and_then(Value::as_str)
                .map(str::to_owned),
            persist: choice
                .get("persist")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            scope: choice
                .get("scope")
                .and_then(Value::as_str)
                .map(str::to_owned),
        })
        .collect()
}

fn questions_of(value: Option<&Value>) -> Vec<Question> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("question").and_then(Value::as_str).is_some())
        .map(|item| Question {
            question: text_of(item, "question"),
            header: text_of(item, "header"),
            multi_select: item
                .get("multiSelect")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            options: item
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|option| (text_of(option, "label"), text_of(option, "description")))
                .collect(),
        })
        .collect()
}

impl Model {
    pub fn add_user(&mut self, text: impl Into<String>) {
        self.entries.push(Entry::new(Kind::User, text));
    }

    /// Applies one decoded event; `false` for unknown or foreign schemas.
    pub fn apply(&mut self, event: &Value) -> bool {
        if event.get("schema").and_then(Value::as_str) != Some(SCHEMA)
            || event.get("schema_version").and_then(Value::as_i64) != Some(SCHEMA_VERSION)
        {
            return false;
        }
        let kind = text_of(event, "type");
        match kind.as_str() {
            "session" | "done" => {
                let session = text_of(event, "session_id");
                if !session.is_empty() {
                    self.session_id = session;
                }
                return true;
            }
            "usage_live" => {
                let count = |key: &str| event.get(key).and_then(Value::as_i64).unwrap_or(0).max(0);
                let id = text_of(event, "message_id");
                self.live.insert(
                    if id.is_empty() { "turn".into() } else { id },
                    (count("input_tokens"), count("output_tokens")),
                );
                return true;
            }
            "usage" => {
                self.live.clear();
                // Each turn is a separate run: the session total is the running sum.
                let count = |key: &str| event.get(key).and_then(Value::as_i64).unwrap_or(0);
                self.usage.input_tokens += count("input_tokens");
                self.usage.output_tokens += count("output_tokens");
                if let Some(cost) = event.get("cost_usd").and_then(Value::as_f64) {
                    *self.usage.cost_usd.get_or_insert(0.0) += cost;
                }
                return true;
            }
            "limits" => {
                if let Some(tokens) = event.get("context_tokens").and_then(Value::as_i64) {
                    self.usage.context_tokens = tokens;
                }
                if let Some(window) = event.get("context_window").and_then(Value::as_i64) {
                    self.usage.context_window = window;
                }
                if let Some(window) = window_of(event.get("primary")) {
                    self.usage.primary = Some(window);
                }
                if let Some(window) = window_of(event.get("secondary")) {
                    self.usage.secondary = Some(window);
                }
                if let Some(plan) = event.get("plan").and_then(Value::as_str) {
                    self.usage.plan = plan.to_owned();
                }
                return true;
            }
            _ => {}
        }
        let entry = match kind.as_str() {
            "message" => Entry::new(Kind::Assistant, text_of(event, "text")),
            "reasoning" => Entry::new(Kind::Reasoning, text_of(event, "text")),
            "notice" => Entry::new(
                if event.get("error").and_then(Value::as_bool).unwrap_or(false) {
                    Kind::Error
                } else {
                    Kind::Notice
                },
                text_of(event, "message"),
            ),
            "error" => Entry::new(Kind::Error, text_of(event, "message")),
            "permission" => {
                let mut entry = Entry::new(Kind::Permission, text_of(event, "summary"));
                entry.tool_id = text_of(event, "id");
                entry.tool_kind = text_of(event, "tool");
                entry.output = tail_of(text_of(event, "detail"), MAX_OUTPUT);
                entry.certain = event
                    .get("certain")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                entry.choices = choices_of(event.get("choices"));
                entry.request_id = text_of(event, "request_id");
                // The same request is not asked twice in one turn.
                if !entry.tool_id.is_empty()
                    && self.entries.iter().any(|other| {
                        other.kind == Kind::Permission
                            && other.tool_id == entry.tool_id
                            && other.decision.is_empty()
                    })
                {
                    return true;
                }
                entry
            }
            "question" => {
                let questions = questions_of(event.get("questions"));
                if questions.is_empty() {
                    return true;
                }
                let mut entry = Entry::new(Kind::Question, "pregunta");
                entry.request_id = text_of(event, "request_id");
                entry.tool_id = text_of(event, "id");
                entry.questions = questions;
                entry
            }
            "tool" => {
                let id = text_of(event, "id");
                // Later events for the same tool update its card in place.
                if !id.is_empty()
                    && let Some(existing) = self
                        .entries
                        .iter_mut()
                        .rev()
                        .find(|e| e.kind == Kind::Tool && e.tool_id == id)
                {
                    existing.status = text_of(event, "status");
                    let output = text_of(event, "output");
                    if !output.is_empty() {
                        existing.output = tail_of(output, MAX_OUTPUT);
                    }
                    let title = text_of(event, "title");
                    if !title.is_empty() {
                        existing.text = title;
                    }
                    existing.revision += 1;
                    return true;
                }
                let mut entry = Entry::new(Kind::Tool, text_of(event, "title"));
                entry.tool_id = id;
                entry.tool_kind = text_of(event, "kind");
                entry.status = text_of(event, "status");
                entry.output = tail_of(text_of(event, "output"), MAX_OUTPUT);
                entry
            }
            other => return matches!(other, "start" | "plan"),
        };
        if entry.text.trim().is_empty() && entry.kind != Kind::Tool {
            return true;
        }
        self.entries.push(entry);
        true
    }

    /// Frees what old entries hold that nobody sees any more (tool output, reasoning): entries
    /// before the last `keep` stay as one-line records. Positions do not change.
    pub fn compact(&mut self, keep: usize) {
        let old = self.entries.len().saturating_sub(keep);
        for entry in &mut self.entries[..old] {
            if entry.kind == Kind::Tool && !entry.output.is_empty() {
                entry.output = String::new();
            }
            if entry.kind == Kind::Reasoning && !entry.text.is_empty() {
                entry.text = String::new();
            }
        }
    }

    /// Tokens the running turn has used so far (input, output); zero when nothing was reported.
    pub fn live_tokens(&self) -> (i64, i64) {
        self.live
            .values()
            .fold((0, 0), |(i, o), (input, output)| (i + input, o + output))
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// True while a live request is waiting for an answer.
    pub fn awaiting_answer(&self) -> bool {
        self.entries.iter().any(Entry::is_open_request)
    }

    /// The turn ended: live requests nobody answered can no longer be answered.
    pub fn expire_live_requests(&mut self) {
        for entry in self.entries.iter_mut().filter(|e| e.is_open_request()) {
            entry.decision = "Ya no esperaba tu respuesta".to_owned();
            entry.revision += 1;
        }
    }

    /// Records what the person answered to a card (an empty label means "not now").
    pub fn decide(&mut self, index: usize, label: &str) {
        if let Some(entry) = self.entries.get_mut(index)
            && matches!(entry.kind, Kind::Permission | Kind::Question)
        {
            entry.decision = if label.is_empty() { "Ahora no" } else { label }.to_owned();
            entry.revision += 1;
        }
    }
}

#[cfg(test)]
#[path = "chat_model_tests.rs"]
mod tests;
