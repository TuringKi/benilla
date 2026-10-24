//! The selection model reads the same guild record as an in-world tabard.

pub(crate) fn emblem(guild: &super::GuildState, id: u32) -> Option<benilla_formats::GuildEmblem> {
    super::guild_emblem(id, guild, super::Ask::Read)
}
