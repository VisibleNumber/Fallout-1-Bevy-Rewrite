use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize)]
pub struct Definition {
    pub id: String,
    pub speaker: String,
    pub start: String,
    pub flags: Vec<String>,
    pub nodes: BTreeMap<String, Node>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Node {
    pub text: String,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub next: Option<String>,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    #[serde(default)]
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Condition {
    pub flag: String,
    pub at_least: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Effect {
    pub flag: String,
    pub value: i32,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Story {
    pub flags: BTreeMap<String, i32>,
    pub completed: BTreeSet<String>,
    pub active: Option<String>,
}

impl Definition {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("dialogue id must not be empty".into());
        }
        if self.start.trim().is_empty() || !self.nodes.contains_key(&self.start) {
            return Err(format!("start node '{}' does not exist", self.start));
        }

        let allowed_flags: BTreeSet<&str> = self.flags.iter().map(String::as_str).collect();
        if allowed_flags.len() != self.flags.len() {
            return Err("definition contains duplicate flag names".into());
        }
        if self.flags.iter().any(|flag| flag.trim().is_empty()) {
            return Err("definition contains an empty flag name".into());
        }

        for (node_id, node) in &self.nodes {
            if node.choices.is_empty() {
                return Err(format!("node '{node_id}' has no choices"));
            }
            let mut choice_ids = BTreeSet::new();
            for choice in &node.choices {
                if choice.id.trim().is_empty() {
                    return Err(format!("node '{node_id}' has an empty choice id"));
                }
                if !choice_ids.insert(choice.id.as_str()) {
                    return Err(format!(
                        "node '{node_id}' has duplicate choice id '{}'",
                        choice.id
                    ));
                }
                if let Some(next) = &choice.next {
                    if !self.nodes.contains_key(next) {
                        return Err(format!(
                            "choice '{}' points to missing node '{next}'",
                            choice.id
                        ));
                    }
                }
                for condition in &choice.conditions {
                    if !allowed_flags.contains(condition.flag.as_str()) {
                        return Err(format!(
                            "choice '{}' uses unknown condition flag '{}'",
                            choice.id, condition.flag
                        ));
                    }
                }
                for effect in &choice.effects {
                    if !allowed_flags.contains(effect.flag.as_str()) {
                        return Err(format!(
                            "choice '{}' uses unknown effect flag '{}'",
                            choice.id, effect.flag
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

impl Story {
    /// Starts the definition at its entry node. A node id in `active` is the
    /// current conversation position; completion clears it and records the quest id.
    pub fn begin(&mut self, definition: &Definition) -> Result<(), String> {
        definition.validate()?;
        self.validate(definition)?;
        if self.completed.contains(&definition.id) {
            return Err(format!("quest '{}' is already completed", definition.id));
        }
        if self.active.is_some() {
            return Err("a conversation is already active".into());
        }
        self.active = Some(definition.start.clone());
        Ok(())
    }

    pub fn available_choices<'a>(
        &self,
        definition: &'a Definition,
    ) -> Result<Vec<&'a Choice>, String> {
        definition.validate()?;
        self.validate(definition)?;
        let node_id = self
            .active
            .as_deref()
            .ok_or_else(|| "no conversation is active".to_string())?;
        let node = definition
            .nodes
            .get(node_id)
            .ok_or_else(|| format!("active node '{node_id}' does not exist"))?;
        Ok(node
            .choices
            .iter()
            .filter(|choice| self.conditions_met(choice))
            .collect())
    }

    pub fn choose(
        &mut self,
        definition: &Definition,
        expected_node: &str,
        choice_id: &str,
    ) -> Result<(), String> {
        definition.validate()?;
        self.validate(definition)?;
        let current_node = self
            .active
            .as_deref()
            .ok_or_else(|| "no conversation is active".to_string())?;
        if current_node != expected_node {
            return Err(format!(
                "stale choice: expected node '{expected_node}', current node is '{current_node}'"
            ));
        }
        let node = definition
            .nodes
            .get(current_node)
            .ok_or_else(|| format!("active node '{current_node}' does not exist"))?;
        let choice = node
            .choices
            .iter()
            .find(|choice| choice.id == choice_id)
            .ok_or_else(|| {
                format!("choice '{choice_id}' does not exist in node '{current_node}'")
            })?;
        if !self.conditions_met(choice) {
            return Err(format!("choice '{choice_id}' is not currently available"));
        }
        if let Some(next) = &choice.next {
            if !definition.nodes.contains_key(next) {
                return Err(format!(
                    "choice '{choice_id}' points to missing node '{next}'"
                ));
            }
        }

        // Stage all state changes only after every check has passed.
        let effects: Vec<(String, i32)> = choice
            .effects
            .iter()
            .map(|effect| (effect.flag.clone(), effect.value))
            .collect();
        let next = choice.next.clone();
        for (flag, value) in effects {
            self.flags.insert(flag, value);
        }
        if let Some(next) = next {
            self.active = Some(next);
        } else {
            self.active = None;
            self.completed.insert(definition.id.clone());
        }
        Ok(())
    }

    pub fn validate(&self, definition: &Definition) -> Result<(), String> {
        let allowed_flags: BTreeSet<&str> = definition.flags.iter().map(String::as_str).collect();
        if let Some(flag) = self
            .flags
            .keys()
            .find(|flag| !allowed_flags.contains(flag.as_str()))
        {
            return Err(format!("story contains unknown flag '{flag}'"));
        }
        if let Some(node) = &self.active {
            if !definition.nodes.contains_key(node) {
                return Err(format!("active node '{node}' does not exist"));
            }
        }
        if let Some(id) = self
            .completed
            .iter()
            .find(|id| id.as_str() != definition.id)
        {
            return Err(format!("story contains unknown completed quest '{id}'"));
        }
        Ok(())
    }

    fn conditions_met(&self, choice: &Choice) -> bool {
        choice.conditions.iter().all(|condition| {
            self.flags.get(&condition.flag).copied().unwrap_or(0) >= condition.at_least
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition() -> Definition {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "start".into(),
            Node {
                text: "The road is dangerous.".into(),
                choices: vec![
                    Choice {
                        id: "ask".into(),
                        label: "Ask about the road".into(),
                        next: Some("road".into()),
                        conditions: vec![],
                        effects: vec![Effect {
                            flag: "asked".into(),
                            value: 1,
                        }],
                    },
                    Choice {
                        id: "bribe".into(),
                        label: "Offer a bribe".into(),
                        next: Some("end".into()),
                        conditions: vec![Condition {
                            flag: "trust".into(),
                            at_least: 2,
                        }],
                        effects: vec![Effect {
                            flag: "paid".into(),
                            value: 1,
                        }],
                    },
                ],
            },
        );
        nodes.insert(
            "road".into(),
            Node {
                text: "Take care.".into(),
                choices: vec![Choice {
                    id: "leave".into(),
                    label: "Leave".into(),
                    next: Some("end".into()),
                    conditions: vec![],
                    effects: vec![],
                }],
            },
        );
        nodes.insert(
            "end".into(),
            Node {
                text: "Goodbye.".into(),
                choices: vec![Choice {
                    id: "finish".into(),
                    label: "Goodbye".into(),
                    next: None,
                    conditions: vec![],
                    effects: vec![Effect {
                        flag: "met_npc".into(),
                        value: 1,
                    }],
                }],
            },
        );
        Definition {
            id: "roadside_help".into(),
            speaker: "Trader".into(),
            start: "start".into(),
            flags: vec![
                "trust".into(),
                "asked".into(),
                "paid".into(),
                "met_npc".into(),
            ],
            nodes,
        }
    }

    #[test]
    fn branching_and_conditions_follow_story_flags() {
        let definition = definition();
        let mut story = Story::default();
        story.begin(&definition).unwrap();
        assert_eq!(story.available_choices(&definition).unwrap().len(), 1);
        story.flags.insert("trust".into(), 2);
        assert_eq!(story.available_choices(&definition).unwrap().len(), 2);
        story.choose(&definition, "start", "ask").unwrap();
        assert_eq!(story.active.as_deref(), Some("road"));
        assert_eq!(story.flags.get("asked"), Some(&1));
    }

    #[test]
    fn stale_and_hidden_choices_do_not_change_state() {
        let definition = definition();
        let mut story = Story::default();
        story.begin(&definition).unwrap();
        let before = story.clone();
        assert!(story.choose(&definition, "road", "ask").is_err());
        assert!(story.choose(&definition, "start", "bribe").is_err());
        assert_eq!(story, before);
    }

    #[test]
    fn invalid_definition_and_effects_are_atomic() {
        let mut definition = definition();
        definition.nodes.get_mut("start").unwrap().choices[0].effects = vec![
            Effect {
                flag: "asked".into(),
                value: 1,
            },
            Effect {
                flag: "not_allowed".into(),
                value: 1,
            },
        ];
        assert!(definition.validate().is_err());
        let mut story = Story::default();
        story.active = Some("start".into());
        let before = story.clone();
        assert!(story.choose(&definition, "start", "ask").is_err());
        assert_eq!(story, before);
    }

    #[test]
    fn completion_records_quest_and_story_round_trips() {
        let definition = definition();
        let mut story = Story::default();
        story.begin(&definition).unwrap();
        story.choose(&definition, "start", "ask").unwrap();
        story.choose(&definition, "road", "leave").unwrap();
        story.choose(&definition, "end", "finish").unwrap();
        assert_eq!(story.active, None);
        assert!(story.completed.contains("roadside_help"));
        assert_eq!(story.flags.get("met_npc"), Some(&1));
        let json = serde_json::to_string(&story).unwrap();
        assert_eq!(serde_json::from_str::<Story>(&json).unwrap(), story);
    }
}
