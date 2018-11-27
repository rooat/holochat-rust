use std::convert::TryFrom;
use hdk::{
    self, 
    AGENT_ADDRESS,
    entry_definition::ValidatingEntryType,
    holochain_core_types::dna::zome::entry_types::Sharing,
    holochain_core_types::error::HolochainError,
    holochain_core_types::json::JsonString,
    holochain_core_types::hash::HashString,
    holochain_core_types::entry::{Entry,entry_type::EntryType},
    error::ZomeApiResult,
};

use super::member;
use super::message;
use super::utils;

#[derive(Serialize, Deserialize, Debug, Clone, DefaultJson)]
pub struct Channel {
    pub name: String,
    pub description: String,
    pub public:bool
}


pub fn public_channel_definition() -> ValidatingEntryType {
    entry!(
        name: "public_channel",
        description: "A channel of which anyone can become a member and post",
        sharing: Sharing::Public,
        native_type: Channel,

        validation_package: || {
            hdk::ValidationPackageDefinition::Entry
        },

        validation: |_channel: Channel, _ctx: hdk::ValidationData| {
            Ok(())
        }
    )
}

pub fn direct_channel_definition() -> ValidatingEntryType {
    entry!(
        name: "direct_channel",
        description: "A channel to which new members can only be added at creation",
        sharing: Sharing::Public,
        native_type: Channel,

        validation_package: || {
            hdk::ValidationPackageDefinition::Entry
        },

        validation: |_channel: Channel, _ctx: hdk::ValidationData| {
            Ok(())
        }
    )
}

// public zome functions

pub fn handle_create_channel(
    name: String,
    description: String,
    initial_members: Vec<member::Member>,
    public: bool,
) -> JsonString {

    let channel = Channel{name, description,public};
    let entry = match public {
        true => Entry::new(EntryType::App("public_channel".into()), channel),
        false => Entry::new(EntryType::App("direct_channel".into()), channel) };

    // good candidate for bundle when that is working
    let channel = hdk::commit_entry(&entry).expect("Could not commit channel");
    hdk::link_entries(&AGENT_ADDRESS,&channel,"rooms")
    .map(|channel_addr|{
            json!({"address": channel_addr}).into()
        })
        .unwrap_or_else(|hdk_err|{hdk_err.into()})
}

pub fn handle_get_my_channels() -> JsonString {
    match get_my_channels() {
        Ok(result) => result.into(),
        Err(hdk_err) => hdk_err.into()
    }
}

pub fn handle_get_members(channel_address: HashString) -> JsonString {
    match get_members(&channel_address) {
        Ok(result) => result.into(),
        Err(hdk_err) => hdk_err.into()
    }
}

pub fn handle_add_members(channel_address: HashString, members: Vec<member::Member>) -> JsonString {
    members.iter().map(|member| {
        utils::link_entries_bidir(&member.hash(), &channel_address, "member_of", "has_member")
    }).collect::<Result<Vec<_>,_>>().map(|_|{
        json!({"success": true}).into()
    }).unwrap_or_else(|hdk_err|{
        hdk_err.into()
    }) 
}

pub fn handle_get_messages(channel_address: String, min_count: u32) -> JsonString {
    match get_messages(channel_address) {
        Ok(result) => result.into(),
        Err(hdk_err) => hdk_err.into()
    }
}

pub fn handle_post_message(channel_address: String, message: message::Message) -> JsonString {
    let channel = from_channel(channel_address);
    let channel_address = hdk::entry_address(&channel).expect("Could not get");
    hdk::commit_entry(&Entry::new(EntryType::App("message".into()), message))
        .and_then(|message_addr| hdk::link_entries(&channel_address, &message_addr, "message_in")) 
        .map(|_|json!({"success": true}).into())
        .unwrap_or_else(|hdk_err|hdk_err.into())
}

fn from_channel(channel_name:String) ->Entry
{
     let channels = get_my_channels()
                    .unwrap();

     let channel  = channels.iter()
                    .filter(|f|f.name==channel_name)
                    .next()
                    .unwrap();
      match channel.public 
      {
        true => Entry::new(EntryType::App("public_channel".into()), channel),
        false => Entry::new(EntryType::App("direct_channel".into()), channel) 
      }
    
    

}

// end public zome functions

fn get_my_channels() -> ZomeApiResult<Vec<Channel>> {
    utils::get_links_and_load(&AGENT_ADDRESS, "rooms").map(|results| {
        results.iter().map(|get_links_result| {
                Channel::try_from(get_links_result.entry.value().clone()).unwrap()
        }).collect()
    })
}

fn get_members(channel_address: &HashString) -> ZomeApiResult<Vec<member::Member>> {
    utils::get_links_and_load(channel_address, "has_member").map(|results| {
        results.iter().map(|get_links_result| {
                member::Member::try_from(get_links_result.entry.value().clone()).unwrap()
        }).collect()
    })
}

fn get_messages(channel_name: String) -> ZomeApiResult<Vec<message::Message>> {
    let channel = from_channel(channel_name);
    let channel_address = hdk::entry_address(&channel).unwrap();
    utils::get_links_and_load(&channel_address, "message_in").map(|results| {
        results.iter().map(|get_links_result| {
                message::Message::try_from(get_links_result.entry.value().clone()).unwrap()
        }).collect()
    })
}