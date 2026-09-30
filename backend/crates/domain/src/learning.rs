//! Learning records: activity revisions, attempts and assessments (#8, #9).
//!
//! The assessment rule lives here, in one place, so the storage adapter and
//! any future provider cannot grade differently.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::errors::DomainError;

/// The exact content a learner was shown. Loaded by the storage adapter
/// only after the ownership predicate has passed.
#[derive(Debug, Clone)]
pub struct ActivityRevision {
    pub id:          Uuid,
    pub activity_id: Uuid,
    pub revision:    i32,
    pub prompt:      String,
    pub options:     Option<Vec<String>>,
    pub answer_key:  Option<Value>,
    pub rubric:      Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentOutcome {
    Correct,
    Partial,
    Incorrect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentMethod {
    ExactMatch,
    Choice,
    Model,
    Manual,
}

impl AssessmentOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Correct => "correct",
            Self::Partial => "partial",
            Self::Incorrect => "incorrect",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "correct" => Some(Self::Correct),
            "partial" => Some(Self::Partial),
            "incorrect" => Some(Self::Incorrect),
            _ => None,
        }
    }
}

impl AssessmentMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExactMatch => "exact_match",
            Self::Choice => "choice",
            Self::Model => "model",
            Self::Manual => "manual",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "exact_match" => Some(Self::ExactMatch),
            "choice" => Some(Self::Choice),
            "model" => Some(Self::Model),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }
}

/// An assessment as stored. `revision` numbers corrections per attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assessment {
    pub outcome:  AssessmentOutcome,
    pub method:   AssessmentMethod,
    pub score:    Option<f64>,
    pub feedback: String,
}

/// What the caller asked to record. `user_id` is the authenticated learner.
#[derive(Debug, Clone)]
pub struct RecordAttempt {
    pub user_id:              Uuid,
    pub activity_revision_id: Uuid,
    pub response:             Value,
    pub assistance:           Vec<Value>,
}

/// The persisted outcome of a submission. `assessment` is `None` while the
/// attempt is pending; pending is never reported as incorrect.
#[derive(Debug, Clone, PartialEq)]
pub struct AttemptReceipt {
    pub attempt_id:           Uuid,
    pub activity_revision_id: Uuid,
    pub submitted_at:         DateTime<Utc>,
    pub assessment:           Option<Assessment>,
}

impl AttemptReceipt {
    /// `pending`, or the outcome of the assessment.
    pub fn status(&self) -> &'static str {
        match &self.assessment {
            None => "pending",
            Some(a) => a.outcome.as_str(),
        }
    }
}

/// Deterministic assessment. Returns `Ok(None)` when the revision carries
/// no answer key the rule can apply, which leaves the attempt pending.
///
/// - A string key with `options`: the response must be one of the options
///   (else `Validation`), method `choice`, exact comparison.
/// - A string key without options: method `exact_match`, compared after
///   trimming, case-folding and collapsing whitespace.
/// - Any other key shape (or none): pending.
pub fn assess(revision: &ActivityRevision, response: &Value) -> Result<Option<Assessment>, DomainError> {
    let Some(Value::String(key)) = &revision.answer_key else {
        return Ok(None);
    };
    let Value::String(answer) = response else {
        return Err(DomainError::Validation("response must be a string for this activity".into()));
    };

    let (method, correct) = match &revision.options {
        Some(options) => {
            if !options.iter().any(|o| o == answer) {
                return Err(DomainError::Validation("response is not one of the options".into()));
            }
            (AssessmentMethod::Choice, answer == key)
        }
        None => (AssessmentMethod::ExactMatch, normalize(answer) == normalize(key)),
    };

    Ok(Some(Assessment {
        outcome: if correct { AssessmentOutcome::Correct } else { AssessmentOutcome::Incorrect },
        method,
        score: Some(if correct { 1.0 } else { 0.0 }),
        feedback: String::new(),
    }))
}

fn normalize(s: &str) -> String {
    s.split_whitespace().map(str::to_lowercase).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn revision(options: Option<Vec<&str>>, answer_key: Option<Value>) -> ActivityRevision {
        ActivityRevision {
            id: Uuid::nil(),
            activity_id: Uuid::nil(),
            revision: 1,
            prompt: "p".into(),
            options: options.map(|o| o.into_iter().map(String::from).collect()),
            answer_key,
            rubric: None,
        }
    }

    #[test]
    fn exact_match_ignores_case_and_whitespace() {
        let rev = revision(None, Some(json!("A mapping  from States to actions")));
        let a = assess(&rev, &json!("  a mapping from states to actions ")).unwrap().unwrap();
        assert_eq!((a.outcome, a.method, a.score), (AssessmentOutcome::Correct, AssessmentMethod::ExactMatch, Some(1.0)));

        let a = assess(&rev, &json!("a table of action values")).unwrap().unwrap();
        assert_eq!((a.outcome, a.score), (AssessmentOutcome::Incorrect, Some(0.0)));
    }

    #[test]
    fn choice_requires_one_of_the_options_and_compares_exactly() {
        let rev = revision(Some(vec!["Reward", "Value"]), Some(json!("Value")));
        assert_eq!(assess(&rev, &json!("Value")).unwrap().unwrap().outcome, AssessmentOutcome::Correct);
        assert_eq!(assess(&rev, &json!("Reward")).unwrap().unwrap().outcome, AssessmentOutcome::Incorrect);
        assert!(matches!(assess(&rev, &json!("value")), Err(DomainError::Validation(_))));
    }

    #[test]
    fn no_answer_key_means_pending() {
        let rev = revision(None, None);
        assert_eq!(assess(&rev, &json!("anything")).unwrap(), None);
        let rev = revision(None, Some(json!({"rubric_version": 2})));
        assert_eq!(assess(&rev, &json!("anything")).unwrap(), None);
    }

    #[test]
    fn non_string_response_to_a_keyed_activity_is_rejected() {
        let rev = revision(None, Some(json!("x")));
        assert!(matches!(assess(&rev, &json!({"text": "x"})), Err(DomainError::Validation(_))));
    }

    #[test]
    fn receipt_status_is_pending_without_assessment() {
        let r = AttemptReceipt { attempt_id: Uuid::nil(), activity_revision_id: Uuid::nil(), submitted_at: Utc::now(), assessment: None };
        assert_eq!(r.status(), "pending");
    }
}
