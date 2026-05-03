/// ACE-compatible protocol types
///
/// ACE (Asheron's Call Emulator) was written before the protocol.xml specification
/// was fully documented. As a result, its deserialization logic differs from what
/// acprotocol generates from protocol.xml in some ways.
///
/// Rather than modifying ACE (the reference implementation), we match its expectations
/// by providing custom serialization here. This module contains structures that serialize
/// in the format ACE expects.
use asheron_rs::enums::{Gender, HeritageGroup};
use asheron_rs::types::PackableList;
use asheron_rs::writers::{ACWritable, ACWriter, write_u32};

/// Wrapper for raw u32 skill advancement class values
/// ACE server defines Inactive = 0, but acprotocol only defines 1, 2, 3
/// This allows us to send 0 for inactive skills
#[derive(Clone, Debug)]
pub struct RawSkillAdvancementClass(pub u32);

impl ACWritable for RawSkillAdvancementClass {
    fn write(&self, writer: &mut dyn ACWriter) -> Result<(), Box<dyn std::error::Error>> {
        write_u32(writer, self.0)?;
        Ok(())
    }
}

/// ACE-compatible CharGenResult
///
/// ACE expects Heritage and Gender as u32 (not u8 as protocol.xml specifies).
/// Additionally, ACE does NOT expect the redundant account string that
/// protocol.xml includes in CharGenResult - it only reads this from the
/// outer Character_SendCharGenResult wrapper message.
#[derive(Clone, Debug)]
pub struct AceCharGenResult {
    pub one: u32,
    pub heritage_group: u32, // u32, not u8 like in acprotocol
    pub gender: u32,         // u32, not u8 like in acprotocol
    pub eyes_strip: u32,
    pub nose_strip: u32,
    pub mouth_strip: u32,
    pub hair_color: u32,
    pub eye_color: u32,
    pub hair_style: u32,
    pub headgear_style: u32,
    pub headgear_color: u32,
    pub shirt_style: u32,
    pub shirt_color: u32,
    pub trousers_style: u32,
    pub trousers_color: u32,
    pub footwear_style: u32,
    pub footwear_color: u32,
    pub skin_shade: u64,
    pub hair_shade: u64,
    pub headgear_shade: u64,
    pub shirt_shade: u64,
    pub trousers_shade: u64,
    pub tootwear_shade: u64,
    pub template_num: u32,
    pub strength: u32,
    pub endurance: u32,
    pub coordination: u32,
    pub quickness: u32,
    pub focus: u32,
    pub self_: u32,
    pub slot: u32,
    pub class_id: u32,
    pub skills: PackableList<RawSkillAdvancementClass>,
    pub name: String,
    pub start_area: u32,
    pub is_admin: u32,
    pub is_envoy: u32,
    pub validation: u32,
}

#[derive(Debug, Clone)]
pub struct AceCharGenConfig {
    pub heritage: HeritageGroup,
    pub gender: Gender,
    pub eyes_strip: u32,
    pub nose_strip: u32,
    pub mouth_strip: u32,
    pub hair_color: u32,
    pub eye_color: u32,
    pub hair_style: u32,
    pub headgear_style: u32,
    pub headgear_color: u32,
    pub shirt_style: u32,
    pub shirt_color: u32,
    pub trousers_style: u32,
    pub trousers_color: u32,
    pub footwear_style: u32,
    pub footwear_color: u32,
    pub skin_shade: u64,
    pub hair_shade: u64,
    pub headgear_shade: u64,
    pub shirt_shade: u64,
    pub trousers_shade: u64,
    pub tootwear_shade: u64,
    pub template_num: u32,
    pub strength: u32,
    pub endurance: u32,
    pub coordination: u32,
    pub quickness: u32,
    pub focus: u32,
    pub self_: u32,
    pub slot: u32,
    pub class_id: u32,
    pub skills: PackableList<RawSkillAdvancementClass>,
    pub name: String,
    pub start_area: u32,
    pub is_admin: u32,
    pub is_envoy: u32,
    pub validation: u32,
}

impl AceCharGenResult {
    pub fn from_generic(config: AceCharGenConfig) -> Self {
        let AceCharGenConfig {
            heritage,
            gender,
            eyes_strip,
            nose_strip,
            mouth_strip,
            hair_color,
            eye_color,
            hair_style,
            headgear_style,
            headgear_color,
            shirt_style,
            shirt_color,
            trousers_style,
            trousers_color,
            footwear_style,
            footwear_color,
            skin_shade,
            hair_shade,
            headgear_shade,
            shirt_shade,
            trousers_shade,
            tootwear_shade,
            template_num,
            strength,
            endurance,
            coordination,
            quickness,
            focus,
            self_,
            slot,
            class_id,
            skills,
            name,
            start_area,
            is_admin,
            is_envoy,
            validation,
        } = config;

        Self {
            one: 1,
            heritage_group: heritage as u32,
            gender: gender as u32,
            eyes_strip,
            nose_strip,
            mouth_strip,
            hair_color,
            eye_color,
            hair_style,
            headgear_style,
            headgear_color,
            shirt_style,
            shirt_color,
            trousers_style,
            trousers_color,
            footwear_style,
            footwear_color,
            skin_shade,
            hair_shade,
            headgear_shade,
            shirt_shade,
            trousers_shade,
            tootwear_shade,
            template_num,
            strength,
            endurance,
            coordination,
            quickness,
            focus,
            self_,
            slot,
            class_id,
            skills,
            name,
            start_area,
            is_admin,
            is_envoy,
            validation,
        }
    }
}

impl ACWritable for AceCharGenResult {
    fn write(&self, writer: &mut dyn ACWriter) -> Result<(), Box<dyn std::error::Error>> {
        // NOTE: ACE does NOT expect the account string here. It reads the account
        // from the outer Character_SendCharGenResult message wrapper instead.

        asheron_rs::writers::write_u32(writer, self.one)?;
        asheron_rs::writers::write_u32(writer, self.heritage_group)?;
        asheron_rs::writers::write_u32(writer, self.gender)?;
        asheron_rs::writers::write_u32(writer, self.eyes_strip)?;
        asheron_rs::writers::write_u32(writer, self.nose_strip)?;
        asheron_rs::writers::write_u32(writer, self.mouth_strip)?;
        asheron_rs::writers::write_u32(writer, self.hair_color)?;
        asheron_rs::writers::write_u32(writer, self.eye_color)?;
        asheron_rs::writers::write_u32(writer, self.hair_style)?;
        asheron_rs::writers::write_u32(writer, self.headgear_style)?;
        asheron_rs::writers::write_u32(writer, self.headgear_color)?;
        asheron_rs::writers::write_u32(writer, self.shirt_style)?;
        asheron_rs::writers::write_u32(writer, self.shirt_color)?;
        asheron_rs::writers::write_u32(writer, self.trousers_style)?;
        asheron_rs::writers::write_u32(writer, self.trousers_color)?;
        asheron_rs::writers::write_u32(writer, self.footwear_style)?;
        asheron_rs::writers::write_u32(writer, self.footwear_color)?;
        asheron_rs::writers::write_u64(writer, self.skin_shade)?;
        asheron_rs::writers::write_u64(writer, self.hair_shade)?;
        asheron_rs::writers::write_u64(writer, self.headgear_shade)?;
        asheron_rs::writers::write_u64(writer, self.shirt_shade)?;
        asheron_rs::writers::write_u64(writer, self.trousers_shade)?;
        asheron_rs::writers::write_u64(writer, self.tootwear_shade)?;
        asheron_rs::writers::write_u32(writer, self.template_num)?;
        asheron_rs::writers::write_u32(writer, self.strength)?;
        asheron_rs::writers::write_u32(writer, self.endurance)?;
        asheron_rs::writers::write_u32(writer, self.coordination)?;
        asheron_rs::writers::write_u32(writer, self.quickness)?;
        asheron_rs::writers::write_u32(writer, self.focus)?;
        asheron_rs::writers::write_u32(writer, self.self_)?;
        asheron_rs::writers::write_u32(writer, self.slot)?;
        asheron_rs::writers::write_u32(writer, self.class_id)?;
        asheron_rs::writers::write_packable_list::<RawSkillAdvancementClass>(writer, &self.skills)?;
        asheron_rs::writers::write_string(writer, &self.name)?;
        asheron_rs::writers::write_u32(writer, self.start_area)?;
        asheron_rs::writers::write_u32(writer, self.is_admin)?;
        asheron_rs::writers::write_u32(writer, self.is_envoy)?;
        asheron_rs::writers::write_u32(writer, self.validation)?;
        Ok(())
    }
}
