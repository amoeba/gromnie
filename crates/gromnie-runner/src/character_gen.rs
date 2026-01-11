use acprotocol::enums::{Gender, HeritageGroup};
use acprotocol::types::PackableList;

use gromnie_client::client::ace_protocol::{
    AceCharGenConfig, AceCharGenResult, RawSkillAdvancementClass,
};

/// Builder for creating ACE-compatible characters with sensible defaults
pub struct CharacterBuilder {
    config: AceCharGenConfig,
}

impl CharacterBuilder {
    /// Create a new character builder with default values
    pub fn new(name: String) -> Self {
        Self {
            config: AceCharGenConfig {
                heritage: HeritageGroup::Aluvian,
                gender: Gender::Male,
                eyes_strip: 0,
                nose_strip: 0,
                mouth_strip: 0,
                hair_color: 0,
                eye_color: 0,
                hair_style: 0,
                headgear_style: 0,
                headgear_color: 0,
                shirt_style: 0,
                shirt_color: 0,
                trousers_style: 0,
                trousers_color: 0,
                footwear_style: 0,
                footwear_color: 0,
                skin_shade: 0,
                hair_shade: 0,
                headgear_shade: 0,
                shirt_shade: 0,
                trousers_shade: 0,
                footwear_shade: 0,
                template_num: 0,
                strength: 10,
                endurance: 10,
                coordination: 10,
                quickness: 10,
                focus: 10,
                self_: 10,
                slot: 0,
                class_id: 0,
                skills: {
                    // Create a list of 55 skill entries, all set to Inactive (0)
                    // The server expects exactly 55 skills in SkillAdvancementClass format
                    // ACE defines Inactive = 0, but acprotocol doesn't have it, so we use RawSkillAdvancementClass
                    let mut skills = vec![];
                    for _ in 0..55 {
                        skills.push(RawSkillAdvancementClass(0));
                    }
                    PackableList {
                        count: 55,
                        list: skills,
                    }
                },
                name,
                start_area: 0,
                is_admin: 0,
                is_envoy: 0,
                validation: 0,
            },
        }
    }

    /// Create a character with a timestamped name for testing
    pub fn new_test_character() -> Self {
        let char_name = format!("TestChar{}", chrono::Utc::now().timestamp() % 10000);
        Self::new(char_name)
    }

    /// Set the character's heritage
    pub fn heritage(mut self, heritage: HeritageGroup) -> Self {
        self.config.heritage = heritage;
        self
    }

    /// Set the character's gender
    pub fn gender(mut self, gender: Gender) -> Self {
        self.config.gender = gender;
        self
    }

    /// Set character attributes (strength, endurance, coordination, quickness, focus, self)
    pub fn attributes(
        mut self,
        strength: u32,
        endurance: u32,
        coordination: u32,
        quickness: u32,
        focus: u32,
        self_: u32,
    ) -> Self {
        self.config.strength = strength;
        self.config.endurance = endurance;
        self.config.coordination = coordination;
        self.config.quickness = quickness;
        self.config.focus = focus;
        self.config.self_ = self_;
        self
    }

    /// Build the character generation result
    pub fn build(self) -> AceCharGenResult {
        AceCharGenResult::from_generic(self.config)
    }
}
