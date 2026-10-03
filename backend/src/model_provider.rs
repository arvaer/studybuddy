//! The operator's model provider over the server's `LlmClient` (19b;
//! docs/phase-2-build.md, 19b). The capsule asks `call/model` with one
//! `agent.v2` request, `(kind system past task turns offered)`, the shape
//! upstream's reference adapter serves (capsule-corp `host/src/claude.rs`),
//! so the learning capsule is written once and runs under either.
//!
//! The server's client is a plain chat completion: one text in, one text
//! out, no tool API, no streaming. So the verbs `offer` lists are laid out
//! in the system prompt as forms the model writes, and the model's whole
//! message is read as the form `act` applies. The kernel sees what it
//! would see from the reference adapter: `{"form": …}` with the adapter's
//! own transcript beside it, or `{"why": …}` when the model wrote no form,
//! which `act` hands the capsule as a refusal it can tell the model about
//! next turn (V1-03a). Tool calling is the provider's, not the kernel's
//! (sdk-surface, "The language").
//!
//! Nothing here reads the database or the provider; `messages` and `reply`
//! are pure, and the endpoint in `routes/internal.rs` does the calling.

use serde_json::{json, Value as Json};

use app::dtos::llm::{LlmMessage, LlmRole};

/// The request kind this provider serves.
pub const KIND: &str = "agent.v2";

/// The chat for `request`: the system prompt with the verbs appended, the
/// task after the earlier tasks, then each turn as what we answered and
/// what came of it, oldest first.
pub fn messages(request: &Json) -> Result<Vec<LlmMessage>, String> {
    let Some([kind, system, past, task, turns, offered]) = request.as_array().map(Vec::as_slice)
    else {
        return Err(format!("{KIND} is (kind system past task turns offered)"));
    };
    if kind != KIND {
        return Err(format!("serves {KIND}, not {kind}"));
    }
    let mut chat = vec![
        LlmMessage {
            role: LlmRole::System,
            content: format!("{}\n\n{}", text(system)?, protocol(offered)?),
        },
        LlmMessage {
            role: LlmRole::User,
            content: opening(past, text(task)?)?,
        },
    ];
    let turns = turns.as_array().ok_or("turns are a list")?;
    for turn in turns.iter().rev() {
        let Some([reply, results]) = turn.as_array().map(Vec::as_slice) else {
            return Err("a turn is (reply results)".into());
        };
        // What we said: the form when there was one, else the message that
        // had none, so the model sees its own words beside the refusal.
        let said = reply["form"]
            .as_str()
            .or_else(|| reply["content"].as_str())
            .ok_or("a turn's reply is this provider's")?;
        chat.push(LlmMessage {
            role: LlmRole::Assistant,
            content: said.to_string(),
        });
        chat.push(LlmMessage {
            role: LlmRole::User,
            content: answer(reply, results)?,
        });
    }
    Ok(chat)
}

/// What the capsule gets for the model's `text`: its one form, or why
/// there is none, beside the text itself.
pub fn reply(text: &str) -> Json {
    match form(text) {
        Some(form) => json!({ "form": form, "content": text }),
        None => json!({
            "why": format!("the model wrote no form; it said: {}", text.trim()),
            "content": text,
        }),
    }
}

/// The verbs as the model is to write them.
fn protocol(offered: &Json) -> Result<String, String> {
    let entries = offered.as_array().ok_or("offered is (offer verbs)")?;
    if entries.is_empty() {
        return Err("no verbs were offered".into());
    }
    let mut out = String::from(
        "You act by writing exactly one of these forms, and nothing else, as your whole message:",
    );
    for entry in entries {
        let Some((name, description, params)) = verb(entry) else {
            return Err(format!(
                "an offered verb is (name description params), not {entry}"
            ));
        };
        let params = if params.is_empty() {
            "…".to_string()
        } else {
            params.join(" ")
        };
        out.push_str(&format!("\n({name} {params}) — {description}"));
    }
    out.push_str(
        "\nOperands are strings in double quotes, or lists in parentheses. No prose, no code fence.",
    );
    Ok(out)
}

/// `(name description)` or `(name description (param…))`.
fn verb(entry: &Json) -> Option<(&str, &str, Vec<&str>)> {
    let entry = entry.as_array()?;
    let name = entry.first()?.as_str()?;
    let description = entry.get(1)?.as_str()?;
    let params = match entry.get(2) {
        None => Vec::new(),
        Some(params) => params
            .as_array()?
            .iter()
            .map(Json::as_str)
            .collect::<Option<Vec<_>>>()?,
    };
    (entry.len() <= 3).then_some((name, description, params))
}

/// The first user message: the task, after the earlier tasks when there
/// are any, oldest first, each with what was reported when it was done.
fn opening(past: &Json, task: &str) -> Result<String, String> {
    let past = past.as_array().ok_or("past is a list")?;
    if past.is_empty() {
        return Ok(task.to_string());
    }
    let mut earlier =
        "Earlier tasks in this session, oldest first, with what you reported when each was done:"
            .to_string();
    for pair in past.iter().rev() {
        let Some([task, summary]) = pair.as_array().map(Vec::as_slice) else {
            return Err("an earlier task is (task summary)".into());
        };
        earlier.push_str(&format!("\n- {} → {}", text(task)?, text(summary)?));
    }
    Ok(format!("{earlier}\n\n{task}"))
}

/// What a turn came to, as the next user message: each result on a line,
/// oldest first, and the reason when the turn's reply had no form.
fn answer(reply: &Json, results: &Json) -> Result<String, String> {
    let results = results.as_array().ok_or("a turn's results are a list")?;
    let mut lines: Vec<String> = results.iter().rev().map(Json::to_string).collect();
    if let Some(why) = reply["why"].as_str() {
        lines.push(why.to_string());
    }
    if lines.is_empty() {
        lines.push("(no result)".to_string());
    }
    Ok(lines.join("\n"))
}

/// The one form in `text`, a code fence stripped if the model added one.
fn form(text: &str) -> Option<String> {
    let mut body = text.trim();
    if let Some(fenced) = body.strip_prefix("```") {
        let after_tag = fenced.find('\n').map_or("", |n| &fenced[n + 1..]);
        body = after_tag.strip_suffix("```").unwrap_or(after_tag).trim();
    }
    (body.starts_with('(') && body.ends_with(')')).then(|| body.to_string())
}

fn text(json: &Json) -> Result<&str, String> {
    json.as_str()
        .ok_or_else(|| format!("expected text, got {json}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(turns: Json) -> Json {
        json!([
            KIND,
            "Coach the learner.",
            [["explain returns", "the learner stated the definition"]],
            "Teach discounting",
            turns,
            [
                [
                    "coach/ask",
                    "Present an activity and wait.",
                    ["path", "prompt"]
                ],
                ["coach/finish", "Finish with a summary."]
            ]
        ])
    }

    #[test]
    fn the_layout_is_system_then_task_then_turns_oldest_first() {
        let turns = json!([
            [{ "why": "the model wrote no form; it said: hello", "content": "hello" }, [["refused", "no form"]]],
            [{ "form": "(coach/ask \"workspaces/w/a\" \"q?\")", "content": "…" }, [["answer", "workspaces/w/a", "42"]]]
        ]);
        let chat = messages(&request(turns)).unwrap();
        let roles: Vec<LlmRole> = chat.iter().map(|m| m.role).collect();
        assert_eq!(
            roles,
            [
                LlmRole::System,
                LlmRole::User,
                LlmRole::Assistant,
                LlmRole::User,
                LlmRole::Assistant,
                LlmRole::User
            ]
        );
        assert!(chat[0].content.starts_with("Coach the learner.\n\n"));
        assert!(chat[0]
            .content
            .contains("(coach/ask path prompt) — Present an activity and wait."));
        assert!(chat[0]
            .content
            .contains("(coach/finish …) — Finish with a summary."));
        assert!(chat[1]
            .content
            .contains("- explain returns → the learner stated the definition"));
        assert!(chat[1].content.ends_with("\n\nTeach discounting"));
        // Oldest turn first: the form we gave, then what came of it.
        assert_eq!(chat[2].content, "(coach/ask \"workspaces/w/a\" \"q?\")");
        assert_eq!(chat[3].content, r#"["answer","workspaces/w/a","42"]"#);
        // A turn with no form shows the model its own words and the reason.
        assert_eq!(chat[4].content, "hello");
        assert_eq!(
            chat[5].content,
            "[\"refused\",\"no form\"]\nthe model wrote no form; it said: hello"
        );
    }

    #[test]
    fn a_malformed_request_is_an_error_not_a_call() {
        assert!(messages(&json!(["agent.v1", "s", [], "t", [], []]))
            .unwrap_err()
            .contains("serves agent.v2"));
        assert!(messages(&json!([KIND, "s", [], "t", []])).is_err());
        assert!(messages(&json!([KIND, "s", [], "t", [], []]))
            .unwrap_err()
            .contains("no verbs"));
        assert!(
            messages(&json!([KIND, "s", [], "t", [], [["only-a-name"]]]))
                .unwrap_err()
                .contains("offered verb")
        );
        assert!(
            messages(&json!([KIND, "s", [], "t", [["not a pair"]], [["v", "d"]]]))
                .unwrap_err()
                .contains("a turn")
        );
    }

    #[test]
    fn the_reply_is_the_form_or_why_there_is_none() {
        assert_eq!(
            reply("  (coach/finish \"done\")\n"),
            json!({ "form": "(coach/finish \"done\")", "content": "  (coach/finish \"done\")\n" })
        );
        assert_eq!(
            reply("```lisp\n(coach/finish \"done\")\n```")["form"],
            "(coach/finish \"done\")"
        );
        let none = reply("I think we should ask about discounting.");
        assert!(none.get("form").is_none());
        assert!(none["why"]
            .as_str()
            .unwrap()
            .contains("I think we should ask"));
    }
}
