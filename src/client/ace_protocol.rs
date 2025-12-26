/// ACE-compatible protocol types
///
/// ACE (Asheron's Call Emulator) was written before the protocol.xml specification
/// was fully documented. As a result, its deserialization logic differs from what
/// acprotocol generates from protocol.xml in some ways.
///
/// Rather than modifying ACE (the reference implementation), we match its expectations
/// by providing custom serialization here. This module contains structures that serialize
/// in the format ACE expects.

use acprotocol::enums::{Gender, HeritageGroup};
use acprotocol::types::PackableList;
use acprotocol::writers::{ACWritable, ACWriter, write_u32};

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
    pub heritage_group: u32,  // u32, not u8 like in acprotocol
    pub gender: u32,           // u32, not u8 like in acprotocol
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
    pub fn from_generic(
        heritage: HeritageGroup,
        gender: Gender,
        eyes_strip: u32,
        nose_strip: u32,
        mouth_strip: u32,
        hair_color: u32,
        eye_color: u32,
        hair_style: u32,
        headgear_style: u32,
        headgear_color: u32,
        shirt_style: u32,
        shirt_color: u32,
        trousers_style: u32,
        trousers_color: u32,
        footwear_style: u32,
        footwear_color: u32,
        skin_shade: u64,
        hair_shade: u64,
        headgear_shade: u64,
        shirt_shade: u64,
        trousers_shade: u64,
        tootwear_shade: u64,
        template_num: u32,
        strength: u32,
        endurance: u32,
        coordination: u32,
        quickness: u32,
        focus: u32,
        self_: u32,
        slot: u32,
        class_id: u32,
        skills: PackableList<RawSkillAdvancementClass>,
        name: String,
        start_area: u32,
        is_admin: u32,
        is_envoy: u32,
        validation: u32,
    ) -> Self {
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
        
        acprotocol::writers::write_u32(writer, self.one)?;
        acprotocol::writers::write_u32(writer, self.heritage_group)?;
        acprotocol::writers::write_u32(writer, self.gender)?;
        acprotocol::writers::write_u32(writer, self.eyes_strip)?;
        acprotocol::writers::write_u32(writer, self.nose_strip)?;
        acprotocol::writers::write_u32(writer, self.mouth_strip)?;
        acprotocol::writers::write_u32(writer, self.hair_color)?;
        acprotocol::writers::write_u32(writer, self.eye_color)?;
        acprotocol::writers::write_u32(writer, self.hair_style)?;
        acprotocol::writers::write_u32(writer, self.headgear_style)?;
        acprotocol::writers::write_u32(writer, self.headgear_color)?;
        acprotocol::writers::write_u32(writer, self.shirt_style)?;
        acprotocol::writers::write_u32(writer, self.shirt_color)?;
        acprotocol::writers::write_u32(writer, self.trousers_style)?;
        acprotocol::writers::write_u32(writer, self.trousers_color)?;
        acprotocol::writers::write_u32(writer, self.footwear_style)?;
        acprotocol::writers::write_u32(writer, self.footwear_color)?;
        acprotocol::writers::write_u64(writer, self.skin_shade)?;
        acprotocol::writers::write_u64(writer, self.hair_shade)?;
        acprotocol::writers::write_u64(writer, self.headgear_shade)?;
        acprotocol::writers::write_u64(writer, self.shirt_shade)?;
        acprotocol::writers::write_u64(writer, self.trousers_shade)?;
        acprotocol::writers::write_u64(writer, self.tootwear_shade)?;
        acprotocol::writers::write_u32(writer, self.template_num)?;
        acprotocol::writers::write_u32(writer, self.strength)?;
        acprotocol::writers::write_u32(writer, self.endurance)?;
        acprotocol::writers::write_u32(writer, self.coordination)?;
        acprotocol::writers::write_u32(writer, self.quickness)?;
        acprotocol::writers::write_u32(writer, self.focus)?;
        acprotocol::writers::write_u32(writer, self.self_)?;
        acprotocol::writers::write_u32(writer, self.slot)?;
        acprotocol::writers::write_u32(writer, self.class_id)?;
        acprotocol::writers::write_packable_list::<RawSkillAdvancementClass>(writer, &self.skills)?;
        acprotocol::writers::write_string(writer, &self.name)?;
        acprotocol::writers::write_u32(writer, self.start_area)?;
        acprotocol::writers::write_u32(writer, self.is_admin)?;
        acprotocol::writers::write_u32(writer, self.is_envoy)?;
        acprotocol::writers::write_u32(writer, self.validation)?;
        Ok(())
    }
}
