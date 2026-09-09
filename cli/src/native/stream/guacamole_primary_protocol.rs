//! Minimal receive-only Guacamole protocol handling for a backend-owned primary.
//! Display payloads are consumed and discarded. Desktop input is never emitted.

const MAX_BUFFER_BYTES: usize = 2 * 1024 * 1024;
const MAX_ELEMENTS: usize = 32;

#[derive(Default)]
pub(super) struct Protocol {
    pending: String,
    primary_id: Option<String>,
}

pub(super) struct Observation {
    pub replies: Vec<String>,
    pub frame_complete: bool,
    pub primary_id: Option<String>,
}

impl Protocol {
    pub fn receive(&mut self, text: &str) -> Result<Observation, &'static str> {
        if self.pending.len().saturating_add(text.len()) > MAX_BUFFER_BYTES {
            return Err("guacamole_primary_protocol_limit");
        }
        self.pending.push_str(text);
        let mut replies = Vec::new();
        let mut frame_complete = false;
        let mut consumed = 0;
        while let Some((size, instruction)) = parse_instruction(&self.pending[consumed..])? {
            consumed += size;
            match instruction.first().map(String::as_str) {
                Some("") if instruction.len() == 2 => {
                    let id = uuid::Uuid::parse_str(&instruction[1])
                        .map_err(|_| "guacamole_primary_identity_invalid")?
                        .to_string();
                    if self
                        .primary_id
                        .as_ref()
                        .is_some_and(|current| current != &id)
                    {
                        return Err("guacamole_primary_identity_changed");
                    }
                    self.primary_id = Some(id);
                }
                Some("sync") if instruction.len() == 2 && decimal(&instruction[1]) => {
                    replies.push(encode(&["sync", &instruction[1]]));
                    frame_complete = true;
                }
                Some("blob") if instruction.len() == 3 && decimal(&instruction[1]) => {
                    replies.push(encode(&["ack", &instruction[1], "consumed", "0"]));
                }
                Some("error" | "disconnect") => return Err("guacamole_primary_provider_closed"),
                Some("sync" | "blob") => return Err("guacamole_primary_protocol_invalid"),
                _ => {}
            }
        }
        self.pending.drain(..consumed);
        Ok(Observation {
            replies,
            frame_complete,
            primary_id: self.primary_id.clone(),
        })
    }
}

fn decimal(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn encode(elements: &[&str]) -> String {
    let mut output = elements
        .iter()
        .map(|element| format!("{}.{}", element.chars().count(), element))
        .collect::<Vec<_>>()
        .join(",");
    output.push(';');
    output
}

type ParsedInstruction = Option<(usize, Vec<String>)>;

fn parse_instruction(input: &str) -> Result<ParsedInstruction, &'static str> {
    let mut offset = 0;
    let mut elements = Vec::new();
    loop {
        let remaining = &input[offset..];
        let Some(dot) = remaining.find('.') else {
            if !remaining.bytes().all(|byte| byte.is_ascii_digit()) || remaining.len() > 10 {
                return Err("guacamole_primary_protocol_invalid");
            }
            return Ok(None);
        };
        let digits = &remaining[..dot];
        if !decimal(digits) || digits.len() > 10 {
            return Err("guacamole_primary_protocol_invalid");
        }
        let length = digits
            .parse::<usize>()
            .map_err(|_| "guacamole_primary_protocol_invalid")?;
        if length > MAX_BUFFER_BYTES || elements.len() >= MAX_ELEMENTS {
            return Err("guacamole_primary_protocol_limit");
        }
        let start = offset + dot + 1;
        let Some((end, delimiter)) = input[start..].char_indices().nth(length) else {
            return Ok(None);
        };
        elements.push(input[start..start + end].to_owned());
        offset = start + end + delimiter.len_utf8();
        match delimiter {
            ';' => return Ok(Some((offset, elements))),
            ',' => {}
            _ => return Err("guacamole_primary_protocol_invalid"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_identity_is_bound_to_the_tunnel_uuid_and_cannot_change() {
        let mut protocol = Protocol::default();
        let value = protocol
            .receive("0.,36.00000000-0000-4000-8000-000000000001;4.sync,1.1;")
            .unwrap();
        assert_eq!(
            value.primary_id.as_deref(),
            Some("00000000-0000-4000-8000-000000000001")
        );
        assert!(value.frame_complete);
        assert_eq!(
            protocol
                .receive("0.,36.00000000-0000-4000-8000-000000000002;")
                .err(),
            Some("guacamole_primary_identity_changed")
        );
    }

    #[test]
    fn fragmented_frames_acknowledge_consumption_without_desktop_input() {
        let mut protocol = Protocol::default();
        let observation = protocol.receive("4.blob,1.7,4.YW").unwrap();
        assert!(observation.replies.is_empty());
        assert!(!observation.frame_complete);
        let observation = protocol
            .receive("Jj;4.sync,3.123;5.mouse,1.1,1.2,1.0;3.key,2.65,1.1;")
            .unwrap();
        assert_eq!(
            observation.replies,
            ["3.ack,1.7,8.consumed,1.0;", "4.sync,3.123;"]
        );
        assert!(observation.frame_complete);
        assert!(protocol.pending.is_empty());
    }

    #[test]
    fn delimiter_content_unicode_and_partial_instructions_are_not_reinterpreted() {
        let mut protocol = Protocol::default();
        let observation = protocol.receive("4.name,4.é;,x;4.sy").unwrap();
        assert!(observation.replies.is_empty());
        assert!(!observation.frame_complete);
        let observation = protocol.receive("nc,1.9;").unwrap();
        assert_eq!(observation.replies, ["4.sync,1.9;"]);
    }

    #[test]
    fn malformed_oversized_and_closed_provider_inputs_fail_without_raw_payloads() {
        for text in ["x.sync;", "4.sync,1.x;", "4.blob,1.x,1.a;", "9999999999.x"] {
            assert!(Protocol::default().receive(text).is_err());
        }
        let closed = Protocol::default().receive("5.error,14.private-secret,3.512;");
        assert_eq!(closed.err(), Some("guacamole_primary_provider_closed"));
        assert_eq!(
            Protocol::default().receive("10.disconnect;").err(),
            Some("guacamole_primary_provider_closed")
        );
        assert_eq!(
            Protocol::default()
                .receive(&"0".repeat(MAX_BUFFER_BYTES + 1))
                .err(),
            Some("guacamole_primary_protocol_limit")
        );
    }
}
