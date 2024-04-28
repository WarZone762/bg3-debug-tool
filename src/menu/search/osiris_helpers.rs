use crate::{osi_fn, wrappers::osiris};

pub(crate) fn give_item(uuid: &str, amount: i32) -> anyhow::Result<()> {
    osi_fn!(TemplateAddTo, uuid, get_host_character()?, amount, 1)?;
    Ok(())
}

pub(crate) fn add_spell(name: &str) -> anyhow::Result<()> {
    osi_fn!(AddSpell, get_host_character()?, name, 1, 1)?;
    Ok(())
}

pub(crate) fn remove_spell(name: &str) -> anyhow::Result<()> {
    osi_fn!(RemoveSpell, get_host_character()?, name, 1)?;
    Ok(())
}

pub(crate) fn add_spell_boost(spell: &str) -> anyhow::Result<()> {
    osi_fn!(
        AddBoosts,
        get_host_character()?,
        format!("UnlockSpell({spell}, AddChildren, d136c5d9-0ff0-43da-acce-a74a07f8d8bf, , )")
            .as_str(),
        "",
        ""
    )?;
    Ok(())
}

pub(crate) fn remove_spell_boost(spell: &str) -> anyhow::Result<()> {
    osi_fn!(
        RemoveBoosts,
        get_host_character()?,
        format!("UnlockSpell({spell}, AddChildren, d136c5d9-0ff0-43da-acce-a74a07f8d8bf, , )")
            .as_str(),
        1,
        "",
        ""
    )?;
    Ok(())
}

pub(crate) fn add_status(name: &str, duration: i32) -> anyhow::Result<()> {
    osi_fn!(ApplyStatus, get_host_character()?, name, duration, 1, "")?;
    Ok(())
}

pub(crate) fn remove_status(name: &str) -> anyhow::Result<()> {
    osi_fn!(RemoveStatus, get_host_character()?, name, "")?;
    Ok(())
}

pub(crate) fn add_passive(name: &str) -> anyhow::Result<()> {
    osi_fn!(AddPassive, get_host_character()?, name)?;
    Ok(())
}

pub(crate) fn remove_passive(name: &str) -> anyhow::Result<()> {
    osi_fn!(RemovePassive, get_host_character()?, name)?;
    Ok(())
}

pub(crate) fn get_host_character() -> anyhow::Result<osiris::Value> {
    Ok(osi_fn!(GetHostCharacter)?.unwrap())
}
