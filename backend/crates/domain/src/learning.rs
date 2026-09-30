//! Learning records: activities, revisions, attempts and assessments
//! (#8, #9, #10, #41).
//!
//! The assessment rule lives here, in one place, so the storage adapter and
//! any future provider cannot grade differently.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::errors::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Recall,
    Explain,
    Apply,
    Diagnose,
}

impl ActivityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Recall => "recall",
            Self::Explain => "explain",
            Self::Apply => "apply",
            Self::Diagnose => "diagnose",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "recall" => Some(Self::Recall),
            "explain" => Some(Self::Explain),
            "apply" => Some(Self::Apply),
            "diagnose" => Some(Self::Diagnose),
            _ => None,
        }
    }
}

/// A stable, owned practice item. Content lives in its revisions.
#[derive(Debug, Clone, PartialEq)]
pub struct Activity {
    pub id:         Uuid,
    pub user_id:    Uuid,
    pub concept_id: Option<Uuid>,
    pub kind:       ActivityKind,
    pub created_at: DateTime<Utc>,
}

/// The exact content a learner was shown. Loaded by the storage adapter
/// only after the ownership predicate has passed.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityRevision {
    pub id:                 Uuid,
    pub activity_id:        Uuid,
    pub revision:           i32,
    pub prompt:             String,
    pub options:            Option<Vec<String>>,
    pub answer_key:         Option<Value>,
    pub rubric:             Option<String>,
    pub source_resource_id: Option<Uuid>,
    pub source_location:    Option<Value>,
    pub created_at:         DateTime<Utc>,
}

/// An activity with the revision a learner selecting it today would see.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityWithRevision {
    pub activity: Activity,
    pub current:  ActivityRevision,
}

/// What an author supplies for one revision (#41). Validated here so the
/// assessment rule in `assess` always finds a shape it understands.
#[derive(Debug, Clone, PartialEq)]
pub struct RevisionContent {
    pub prompt:             String,
    pub options:            Option<Vec<String>>,
    pub answer_key:         Option<Value>,
    pub rubric:             Option<String>,
    pub source_resource_id: Option<Uuid>,
    pub source_location:    Option<Value>,
}

pub const PROMPT_MAX_LEN: usize = 10_000;

impl RevisionContent {
    /// - The prompt is non-empty after trimming and at most `PROMPT_MAX_LEN`.
    /// - Options, if given, are at least two distinct non-empty strings, and
    ///   the answer key must then be a string naming one of them.
    /// - A source location needs a source resource to locate within.
    pub fn validate(&self) -> Result<(), DomainError> {
        let prompt = self.prompt.trim();
        if prompt.is_empty() {
            return Err(DomainError::Validation("prompt is required".into()));
        }
        if prompt.len() > PROMPT_MAX_LEN {
            return Err(DomainError::Validation(format!("prompt exceeds {PROMPT_MAX_LEN} characters")));
        }
        if let Some(options) = &self.options {
            if options.len() < 2 {
                return Err(DomainError::Validation("options need at least two entries".into()));
            }
            if options.iter().any(|o| o.trim().is_empty()) {
                return Err(DomainError::Validation("options must not be blank".into()));
            }
            let mut seen = std::collections::HashSet::new();
            if !options.iter().all(|o| seen.insert(o)) {
                return Err(DomainError::Validation("options must be distinct".into()));
            }
            match &self.answer_key {
                Some(Value::String(key)) if options.contains(key) => {}
                _ => return Err(DomainError::Validation("answerKey must be one of the options".into())),
            }
        }
        if self.source_location.is_some() && self.source_resource_id.is_none() {
            return Err(DomainError::Validation("sourceLocation needs a sourceResourceId".into()));
        }
        Ok(())
    }
}

/// Create an activity with its first revision. `user_id` owns the activity
/// and must own the linked concept and source resource, if any.
#[derive(Debug, Clone)]
pub struct NewActivity {
    pub user_id:    Uuid,
    pub kind:       ActivityKind,
    pub concept_id: Option<Uuid>,
    pub content:    RevisionContent,
}

/// Add a revision to an owned activity. Earlier revisions are untouched.
#[derive(Debug, Clone)]
pub struct NewRevision {
    pub user_id:     Uuid,
    pub activity_id: Uuid,
    pub content:     RevisionContent,
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
/// `request_key` is chosen by the client per submission and is unique per
/// learner; resending it replays the receipt instead of recording again.
#[derive(Debug, Clone)]
pub struct RecordAttempt {
    pub user_id:              Uuid,
    pub request_key:          String,
    pub activity_revision_id: Uuid,
    pub response:             Value,
    pub assistance:           Vec<Value>,
}

impl RecordAttempt {
    /// The idempotency rule (#10): a resent key is a replay only when the
    /// revision, the response and the assistance are all identical. JSON
    /// equality is structural, so key order does not matter but `"1"` and
    /// `1` do.
    pub fn same_payload(&self, activity_revision_id: Uuid, response: &Value, assistance: &Value) -> bool {
        self.activity_revision_id == activity_revision_id
            && &self.response == response
            && Value::Array(self.assistance.clone()) == *assistance
    }
}

/// The result of `record`: the receipt, and whether it was replayed from an
/// earlier submission with the same request key rather than newly written.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    pub receipt:  AttemptReceipt,
    pub replayed: bool,
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
            source_resource_id: None,
            source_location: None,
            created_at: Utc::now(),
        }
    }

    fn content(options: Option<Vec<&str>>, answer_key: Option<Value>) -> RevisionContent {
        RevisionContent {
            prompt: "What does a policy specify?".into(),
            options: options.map(|o| o.into_iter().map(String::from).collect()),
            answer_key,
            rubric: None,
            source_resource_id: None,
            source_location: None,
        }
    }

    #[test]
    fn revision_content_rules() {
        assert!(content(None, None).validate().is_ok());
        assert!(content(None, Some(json!("policy"))).validate().is_ok());
        assert!(content(Some(vec!["Reward", "Value"]), Some(json!("Value"))).validate().is_ok());

        let bad = [
            RevisionContent { prompt: "  ".into(), ..content(None, None) },
            RevisionContent { prompt: "x".repeat(PROMPT_MAX_LEN + 1), ..content(None, None) },
            content(Some(vec!["Only"]), Some(json!("Only"))),
            content(Some(vec!["A", " "]), Some(json!("A"))),
            content(Some(vec!["A", "A"]), Some(json!("A"))),
            content(Some(vec!["A", "B"]), None),
            content(Some(vec!["A", "B"]), Some(json!("C"))),
            content(Some(vec!["A", "B"]), Some(json!(["A"]))),
            RevisionContent { source_location: Some(json!({"page": 3})), ..content(None, None) },
        ];
        for c in bad {
            assert!(matches!(c.validate(), Err(DomainError::Validation(_))), "{c:?}");
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
    fn same_payload_compares_revision_response_and_assistance() {
        let rev = Uuid::new_v4();
        let cmd = RecordAttempt {
            user_id: Uuid::nil(),
            request_key: "k".into(),
            activity_revision_id: rev,
            response: json!({"a": 1, "b": [1, 2]}),
            assistance: vec![json!("hint")],
        };
        assert!(cmd.same_payload(rev, &json!({"b": [1, 2], "a": 1}), &json!(["hint"])));
        assert!(!cmd.same_payload(Uuid::new_v4(), &cmd.response, &json!(["hint"])));
        assert!(!cmd.same_payload(rev, &json!({"a": "1", "b": [1, 2]}), &json!(["hint"])));
        assert!(!cmd.same_payload(rev, &cmd.response, &json!([])));
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
