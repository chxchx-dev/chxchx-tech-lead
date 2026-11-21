//! A person's answer to what an agent asked during a job: a permission card (allow or deny) or a
//! question (one answer per question).

use crate::{ErrorCode, JobError};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnswerBehavior {
    Allow,
    Deny,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobAnswer {
    /// The `request_id` of the permission or question event.
    pub request_id: String,
    /// For a permission card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior: Option<AnswerBehavior>,
    /// How long an allowed permission lasts, when the agent offers a choice (e.g. `session`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// For a question: the chosen answer to each question, by question.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<Map<String, Value>>,
}

const MAX_ANSWER_TEXT: usize = 4_000;

impl JobAnswer {
    pub fn validate(&self) -> Result<(), JobError> {
        let invalid = |message: &str| Err(JobError::new(ErrorCode::InvalidRequest, message));
        let id = &self.request_id;
        if id.is_empty() || id.len() > 200 || id.chars().any(char::is_control) {
            return invalid("request_id no válido");
        }
        match (&self.behavior, &self.answers) {
            (Some(_), None) | (None, Some(_)) => {}
            _ => return invalid("Una respuesta lleva `behavior` (permiso) o `answers` (pregunta)"),
        }
        if let Some(scope) = &self.scope
            && (self.behavior.is_none()
                || scope.is_empty()
                || scope.len() > 64
                || !scope
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        {
            return invalid("scope no válido");
        }
        if let Some(answers) = &self.answers {
            let size: usize = answers
                .iter()
                .map(|(question, answer)| question.len() + answer.to_string().len())
                .sum();
            if answers.is_empty() || size > MAX_ANSWER_TEXT {
                return invalid("Respuestas vacías o demasiado largas");
            }
            if !answers.values().all(Value::is_string) {
                return invalid("Cada respuesta debe ser texto");
            }
        }
        Ok(())
    }

    /// The answer line the product's agent bridge understands (the same Studio sends).
    pub fn to_bridge(&self) -> Value {
        let mut line = json!({ "type": "answer", "request_id": self.request_id });
        if let Some(answers) = &self.answers {
            line["answers"] = Value::Object(answers.clone());
        }
        if let Some(behavior) = self.behavior {
            line["behavior"] = json!(behavior);
        }
        if let Some(scope) = &self.scope {
            line["scope"] = json!(scope);
        }
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn permission(behavior: AnswerBehavior) -> JobAnswer {
        JobAnswer {
            request_id: "req-1".into(),
            behavior: Some(behavior),
            scope: None,
            answers: None,
        }
    }

    #[test]
    fn a_permission_answer_becomes_the_line_studio_sends() {
        let mut allow = permission(AnswerBehavior::Allow);
        allow.scope = Some("session".into());
        assert!(allow.validate().is_ok());
        assert_eq!(
            allow.to_bridge(),
            json!({"type": "answer", "request_id": "req-1", "behavior": "allow", "scope": "session"})
        );
    }

    #[test]
    fn a_question_answer_carries_one_text_per_question() {
        let answer: JobAnswer = serde_json::from_value(json!({
            "request_id": "q-1", "answers": { "¿Qué base de datos?": "PostgreSQL" }
        }))
        .unwrap();
        assert!(answer.validate().is_ok());
        assert_eq!(
            answer.to_bridge()["answers"]["¿Qué base de datos?"],
            "PostgreSQL"
        );
        assert!(answer.to_bridge().get("behavior").is_none());
    }

    #[test]
    fn ambiguous_empty_or_odd_answers_are_refused() {
        let both: JobAnswer = serde_json::from_value(json!({
            "request_id": "r", "behavior": "allow", "answers": {"q": "a"}
        }))
        .unwrap();
        assert!(both.validate().is_err());
        let neither: JobAnswer = serde_json::from_value(json!({ "request_id": "r" })).unwrap();
        assert!(neither.validate().is_err());
        let mut bad_scope = permission(AnswerBehavior::Allow);
        bad_scope.scope = Some("all; rm".into());
        assert!(bad_scope.validate().is_err());
        let not_text: JobAnswer =
            serde_json::from_value(json!({ "request_id": "r", "answers": {"q": 1} })).unwrap();
        assert!(not_text.validate().is_err());
        assert!(
            serde_json::from_value::<JobAnswer>(json!({
                "request_id": "r", "behavior": "allow", "command": "x"
            }))
            .is_err(),
            "unknown fields are refused"
        );
    }
}
