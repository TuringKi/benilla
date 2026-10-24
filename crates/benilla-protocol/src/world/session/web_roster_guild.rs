//! Guild records for the selected character's tabard while the world link is parked.

use super::*;

impl WorldSession {
    pub(super) fn query_roster_guilds(&mut self, characters: &[Character]) -> Result<()> {
        let mut queried = std::collections::HashSet::new();
        for guild in characters.iter().map(|c| c.guild_id).filter(|&g| g != 0) {
            if queried.insert(guild) {
                // This is an authenticated-link query, independent of entering a character.
                // Do not wait for it: a slow/missing crest must never block the character list.
                self.send(opcode::CMSG_GUILD_QUERY, &messages::guild_query(guild))?;
            }
        }
        Ok(())
    }

    /// Guild replies consumed by the character-select link poll, for the app's normal cache.
    pub fn take_roster_guild_responses(&mut self) -> Vec<messages::GuildQueryResponse> {
        std::mem::take(&mut self.link.roster_guild_responses)
    }
}
