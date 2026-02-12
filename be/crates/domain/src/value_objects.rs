use serde::{Deserialize, Serialize};

use crate::errors::DomainError;

// ─── Email ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    pub fn new(raw: impl Into<String>) -> Result<Self, DomainError> {
        let s = raw.into().trim().to_lowercase();
        if s.is_empty() || !s.contains('@') || !s.contains('.') {
            return Err(DomainError::Validation(format!("invalid email: {s}")));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ─── RuState ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuState {
    Introduced,
    Reinforced,
    Unstable,
    Stabilizing,
    Stable,
    Superseded,
}

impl RuState {
    /// Valid forward transitions in the state machine.
    pub fn can_transition_to(&self, next: &RuState) -> bool {
        use RuState::*;
        matches!(
            (self, next),
            (Introduced, Reinforced)
                | (Reinforced, Stabilizing)
                | (Reinforced, Unstable)
                | (Stabilizing, Stable)
                | (Stabilizing, Unstable)
                | (Unstable, Reinforced)
                | (Stable, Superseded)
                | (_, Unstable)   // can degrade from any active state
        )
    }
}

impl std::fmt::Display for RuState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RuState::Introduced  => "introduced",
            RuState::Reinforced  => "reinforced",
            RuState::Unstable    => "unstable",
            RuState::Stabilizing => "stabilizing",
            RuState::Stable      => "stable",
            RuState::Superseded  => "superseded",
        };
        write!(f, "{s}")
    }
}

impl TryFrom<&str> for RuState {
    type Error = DomainError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "introduced"  => Ok(RuState::Introduced),
            "reinforced"  => Ok(RuState::Reinforced),
            "unstable"    => Ok(RuState::Unstable),
            "stabilizing" => Ok(RuState::Stabilizing),
            "stable"      => Ok(RuState::Stable),
            "superseded"  => Ok(RuState::Superseded),
            other => Err(DomainError::Validation(format!("unknown RU state: {other}"))),
        }
    }
}

// ─── QuestionType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    Recall,
    Application,
    Disambiguation,
}

impl std::fmt::Display for QuestionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            QuestionType::Recall        => "recall",
            QuestionType::Application   => "application",
            QuestionType::Disambiguation => "disambiguation",
        };
        write!(f, "{s}")
    }
}

impl TryFrom<&str> for QuestionType {
    type Error = DomainError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "recall"         => Ok(QuestionType::Recall),
            "application"    => Ok(QuestionType::Application),
            "disambiguation" => Ok(QuestionType::Disambiguation),
            other => Err(DomainError::Validation(format!("unknown question type: {other}"))),
        }
    }
}

// ─── StudySessionType ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudySessionType {
    Reading,
    Video,
    Quiz,
}

impl std::fmt::Display for StudySessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            StudySessionType::Reading => "reading",
            StudySessionType::Video   => "video",
            StudySessionType::Quiz    => "quiz",
        };
        write!(f, "{s}")
    }
}

impl TryFrom<&str> for StudySessionType {
    type Error = DomainError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "reading" => Ok(StudySessionType::Reading),
            "video"   => Ok(StudySessionType::Video),
            "quiz"    => Ok(StudySessionType::Quiz),
            other => Err(DomainError::Validation(format!("unknown session type: {other}"))),
        }
    }
}

// ─── ResourceType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Pdf,
    Video,
    Article,
    Lecture,
    Textbook,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ResourceType::Pdf      => "pdf",
            ResourceType::Video    => "video",
            ResourceType::Article  => "article",
            ResourceType::Lecture  => "lecture",
            ResourceType::Textbook => "textbook",
        };
        write!(f, "{s}")
    }
}

impl TryFrom<&str> for ResourceType {
    type Error = DomainError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "pdf"      => Ok(ResourceType::Pdf),
            "video"    => Ok(ResourceType::Video),
            "article"  => Ok(ResourceType::Article),
            "lecture"  => Ok(ResourceType::Lecture),
            "textbook" => Ok(ResourceType::Textbook),
            other => Err(DomainError::Validation(format!("unknown resource type: {other}"))),
        }
    }
}
