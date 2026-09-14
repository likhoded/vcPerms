use pumpkin_plugin_api::command::{CommandError, CommandSender};

use crate::cmd::{msg, need, save};
use crate::state::{with_store, with_store_mut};

pub fn handle(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    let name = need(args, 0, "track")?;
    if args.len() == 1 {
        return info(sender, name);
    }
    match args[1].to_ascii_lowercase().as_str() {
        "info" | "editor" => info(sender, name),
        "append" => {
            let group = need(args, 2, "group")?;
            with_store_mut(|store| {
                if store.group(group).is_none() {
                    return Err(fail(format!("group '{group}' does not exist")));
                }
                let track = store
                    .track_mut(name)
                    .ok_or_else(|| fail(format!("track '{name}' does not exist")))?;
                track.append(group);
                store.touch_track(name);
                Ok(())
            })?;
            save();
            msg(sender, &format!("&aAppended {group} to {name}"));
            Ok(())
        }
        "insert" => {
            let group = need(args, 2, "group")?;
            let pos: usize = need(args, 3, "position")?
                .parse()
                .map_err(|_| fail("position must be a number"))?;
            with_store_mut(|store| {
                if store.group(group).is_none() {
                    return Err(fail(format!("group '{group}' does not exist")));
                }
                let track = store
                    .track_mut(name)
                    .ok_or_else(|| fail(format!("track '{name}' does not exist")))?;
                let idx = pos.saturating_sub(1);
                track.insert(group, idx);
                store.touch_track(name);
                Ok(())
            })?;
            save();
            msg(sender, &format!("&aInserted {group} into {name} at {pos}"));
            Ok(())
        }
        "remove" => {
            let group = need(args, 2, "group")?;
            let ok = with_store_mut(|store| {
                let track = store
                    .track_mut(name)
                    .ok_or_else(|| fail(format!("track '{name}' does not exist")))?;
                let ok = track.remove(group);
                store.touch_track(name);
                Ok(ok)
            })?;
            save();
            if ok {
                msg(sender, &format!("&aRemoved {group} from {name}"));
            } else {
                msg(sender, &format!("&e{group} was not on {name}"));
            }
            Ok(())
        }
        "clear" => {
            with_store_mut(|store| {
                let track = store
                    .track_mut(name)
                    .ok_or_else(|| fail(format!("track '{name}' does not exist")))?;
                track.groups.clear();
                store.touch_track(name);
                Ok(())
            })?;
            save();
            msg(sender, &format!("&aCleared track {name}"));
            Ok(())
        }
        "rename" => {
            let to = need(args, 2, "new name")?;
            with_store_mut(|s| s.rename_track(name, to)).map_err(fail)?;
            save();
            msg(sender, &format!("&aRenamed track {name} -> {to}"));
            Ok(())
        }
        "clone" => {
            let to = need(args, 2, "new name")?;
            with_store_mut(|s| s.clone_track(name, to)).map_err(fail)?;
            save();
            msg(sender, &format!("&aCloned track {name} -> {to}"));
            Ok(())
        }
        other => {
            msg(sender, &format!("&cUnknown track subcommand '{other}'"));
            Ok(())
        }
    }
}

pub fn create(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    let name = need(args, 0, "track")?;
    let created = with_store_mut(|s| s.ensure_track(name));
    save();
    if created {
        msg(sender, &format!("&aCreated track {name}"));
    } else {
        msg(sender, &format!("&eTrack {name} already exists"));
    }
    Ok(())
}

pub fn delete(sender: &CommandSender, args: &[String]) -> Result<(), CommandError> {
    let name = need(args, 0, "track")?;
    with_store_mut(|s| s.delete_track(name)).map_err(fail)?;
    save();
    msg(sender, &format!("&aDeleted track {name}"));
    Ok(())
}

pub fn list(sender: &CommandSender) {
    let lines = with_store(|s| {
        s.tracks()
            .map(|t| format!("&7- &f{} &8{}", t.name, t.display()))
            .collect::<Vec<_>>()
    });
    msg(sender, &format!("&aTracks &7({})", lines.len()));
    if lines.is_empty() {
        msg(sender, "&7(none)");
    }
    for line in lines {
        msg(sender, &line);
    }
}

fn info(sender: &CommandSender, name: &str) -> Result<(), CommandError> {
    let text = with_store(|store| {
        store
            .track(name)
            .map(|t| format!("&b{} \n&7{}", t.name, t.display()))
            .unwrap_or_else(|| format!("&ctrack '{name}' does not exist"))
    });
    msg(sender, &text);
    Ok(())
}

fn fail(e: impl ToString) -> CommandError {
    CommandError::CommandFailed(crate::util::chat(&format!("&c{}", e.to_string())))
}
