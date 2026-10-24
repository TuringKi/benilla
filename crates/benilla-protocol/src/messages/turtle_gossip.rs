//! Extended gossip menus used by Turtle item scripts. Consume every option before reading
//! quests: clamping the wire count to 15 makes option 16 look like the quest count.

use std::io;

use super::gossip::{GossipOption, QuestOption};
use crate::wire::{capacity_hint, read_cstring, read_u32_le, read_u64_le, read_u8};

pub(super) fn read_extended_gossip_message(
    r: &mut &[u8],
) -> io::Result<(u64, u32, Vec<GossipOption>, Vec<QuestOption>)> {
    let guid = read_u64_le(r)?;
    let text_id = read_u32_le(r)?;
    let option_count = read_u32_le(r)?;
    let mut options = Vec::with_capacity(capacity_hint(option_count, 32));
    for _ in 0..option_count {
        options.push(GossipOption {
            index: read_u32_le(r)?,
            icon: read_u8(r)?,
            coded: read_u8(r)? & 1 != 0,
            message: read_cstring(r)?,
        });
    }
    let quest_count = read_u32_le(r)?;
    let mut quests = Vec::with_capacity(capacity_hint(quest_count, 32));
    for _ in 0..quest_count {
        quests.push(QuestOption {
            quest_id: read_u32_le(r)?,
            icon: read_u32_le(r)?,
            level: read_u32_le(r)?,
            title: read_cstring(r)?,
        });
    }
    Ok((guid, text_id, options, quests))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(count: u32, quest: bool) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0x4000_0000_0000_0001u64.to_le_bytes());
        body.extend_from_slice(&100u32.to_le_bytes());
        body.extend_from_slice(&count.to_le_bytes());
        for index in 0..count {
            body.extend_from_slice(&index.to_le_bytes());
            body.extend_from_slice(&[2, u8::from(index == 31)]);
            body.extend_from_slice(format!("Destination {index}\0").as_bytes());
        }
        body.extend_from_slice(&u32::from(quest).to_le_bytes());
        if quest {
            for value in [42u32, 2, 60] {
                body.extend_from_slice(&value.to_le_bytes());
            }
            body.extend_from_slice(b"A quest\0");
        }
        body
    }

    #[test]
    fn seventeen_option_item_menu_is_not_misread_as_quests() {
        let bytes = menu(17, false);
        let mut r = bytes.as_slice();
        let (guid, text, options, quests) = read_extended_gossip_message(&mut r).unwrap();
        assert_eq!(guid, 0x4000_0000_0000_0001);
        assert_eq!(text, 100);
        assert_eq!(options.len(), 17);
        assert_eq!(options[16].index, 16);
        assert!(quests.is_empty());
        assert!(r.is_empty());
        #[cfg(not(target_arch = "wasm32"))]
        assert!(super::super::gossip::read_gossip_message(&mut bytes.as_slice()).is_err());
    }

    #[test]
    fn thirty_two_options_preserve_the_following_quest_and_coded_flag() {
        let bytes = menu(32, true);
        let mut r = bytes.as_slice();
        let (_, _, options, quests) = read_extended_gossip_message(&mut r).unwrap();
        assert_eq!(options.len(), 32);
        assert!(options[31].coded);
        assert_eq!(quests[0].quest_id, 42);
        assert_eq!(quests[0].title, "A quest");
        assert!(r.is_empty());
    }

    #[test]
    fn truncated_extended_menu_is_rejected() {
        let mut bytes = menu(17, false);
        bytes.pop();
        assert!(read_extended_gossip_message(&mut bytes.as_slice()).is_err());
    }
}
