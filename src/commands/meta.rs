use crate::{
    ShardManagerContainer,
    ConnectionPool,
    utils::{
        database::obtain_pool,
        basic_functions::seconds_to_days,
    },
    Uptime,
};
use std::{
    fs::{
        File,
        read_to_string,
    },
    io::prelude::*,
    process::id,
    time::{
        //Duration,
        Instant,
    },
};

use sqlx;
use futures::TryStreamExt;
use futures::stream::StreamExt;

use serenity::{
    prelude::Context,
    model::{
        channel::Message,
        Permissions,
        //channel::ReactionType,
    },
    client::bridge::gateway::ShardId,
    framework::standard::{
        Args,
        CommandResult,
        macros::command,
    },
};
use num_format::{
    Locale,
    ToFormattedString,
};
use toml::Value;
use tokio::process::Command;
use serde_json::json;
use walkdir::WalkDir;


#[command] // Sets up a command
#[aliases("pong", "latency")] // Sets up aliases to that command.
#[description = "Sends the latency of the bot to the shards."] // Sets a description to be used for the help command. You can also use docstrings.

// All command functions must take a Context and Message type parameters.
// Optionally they may also take an Args type parameter for command arguments.
// They must also return CommandResult.
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    // The shard manager is an interface for mutating, stopping, restarting, and
    // retrieving information about shards.
    let data = ctx.data.read().await;
    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(v) => v,
        None => {
            msg.reply(ctx, "There was a problem getting the shard manager").await?;

            return Ok(());
        },
    };

    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;

    // Shards are backed by a "shard runner" responsible for processing events
    // over the shard, so we'll get the information about the shard runner for
    // the shard this command was sent over.
    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            msg.reply(ctx,  "No shard found").await?;

            return Ok(());
        },
    };
   
    let shard_latency = match runner.latency {
        Some(ms) => format!("{:.2}ms", ms.as_micros() as f32 / 1000.0),
        _ => String::new(),
    };

    let map = json!({"content" : "Calculating latency..."});

    let now = Instant::now();
    let mut message = ctx.http.send_message(msg.channel_id.0, &map).await?;
    let rest_latency = now.elapsed().as_millis();

    message.edit(ctx, |m| m.content(format!("Ping?\nGateway: {}\nREST: {}ms", shard_latency, rest_latency))).await?;

    Ok(())
}

/// This command just sends an invite of the bot with the required permissions.
#[command]
async fn invite(ctx: &Context, msg: &Message) -> CommandResult {
    // Sets up the permissions
    let mut permissions = Permissions::empty();
    permissions.set(Permissions::KICK_MEMBERS, true);
    permissions.set(Permissions::BAN_MEMBERS, true);
    permissions.set(Permissions::MANAGE_CHANNELS, true);
    permissions.set(Permissions::ADD_REACTIONS, true);
    permissions.set(Permissions::VIEW_AUDIT_LOG, true);
    permissions.set(Permissions::READ_MESSAGES, true);
    permissions.set(Permissions::SEND_MESSAGES, true);
    permissions.set(Permissions::MANAGE_MESSAGES, true);
    permissions.set(Permissions::EMBED_LINKS, true);
    permissions.set(Permissions::ATTACH_FILES, true);
    permissions.set(Permissions::READ_MESSAGE_HISTORY, true);
    permissions.set(Permissions::USE_EXTERNAL_EMOJIS, true);
    permissions.set(Permissions::CONNECT, true);
    permissions.set(Permissions::SPEAK, true);
    permissions.set(Permissions::MOVE_MEMBERS, true);
    permissions.set(Permissions::MANAGE_ROLES, true);
    permissions.set(Permissions::MANAGE_WEBHOOKS, true);
    permissions.set(Permissions::MENTION_EVERYONE, true);

    // Creates the invite link for the bot with the permissions specified earlier.
    // Error handling in rust i so nice.
    let url = match ctx.cache.current_user().await.invite_url(ctx, permissions).await {
        Ok(v) => v,
        Err(why) => {
            println!("Error creating invite url: {:?}", why);

            return Ok(()); // Prematurely finish the command function.
        }
    };
    
    msg.channel_id.send_message(ctx, |m| {
        m.embed( |e| {
            e.title("Invite Link");
            e.url(url);
            e.description("Keep in mind, this bot is still in pure developement, so not all of this mentioned features are implemented.\n\n__**Reason for each permission**__");
            e.fields(vec![
                     ("Move Members", "To automatically move members to the current music room (as long as there's people already listening there).", true),
                     ("Attach Files", "For some of the booru commands.\nFor an automatic text file to be sent when a message is too long.", true),
                     ("Read Messages", "So the bot can read the messages to know when a command was invoked and such.", true),
                     ("Manage Messages", "Be able to clear reactions of timed out paginations.\nClear moderation command.", true),
                     ("Manage Channels", "Be able to mute members on the channel without having to create a role for it.", true),
                     ("Manage Webhooks", "For all the commands that can be ran on a schedule, so it's more efficient.", true),
                     ("Manage Roles", "Be able to give a stream notification role.\nMute moderation command.", true),
                     ("Read Message History", "This is a required permission for every paginated command.", true),
                     ("Use External Emojis", "For all the commands that use emojis for better emphasis.", true),
                     ("View Audit Log", "To be able to have a more feature rich logging to a channel.", true),
                     ("Add Reactions", "To be able to add reactions for all the paginated commands.", true),
                     ("Mention Everyone", "To be able to mention the livestream notification role.", true),
                     ("Send Messages", "So the bot can send the messages it needs to send.", true),
                     ("Speak", "To be able to play music on that voice channel.", true),
                     ("Embed Links", "For the tags to be able to embed images.", true),
                     ("Connect", "To be able to connect to a voice channel.", true),
                     ("Kick Members", "Kick/GhostBan moderation command.", true),
                     ("Ban Members", "Ban moderation command.", true),
            ]);
            e
        });

        m
    }).await?;
    Ok(())
}

#[command]
#[help_available(false)] // makes it not show up on the help menu
#[owners_only] // to only allow the owner of the bot to use this command
//#[min_args(3)] // Sets the minimum ammount of arguments the command requires to be ran. This is used to trigger the `NotEnoughArguments` error.
// Testing command, please ignore.
async fn test(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut embed_json = args.message().to_string();
    dbg!(&embed_json);

    if embed_json.starts_with("```json") {
        embed_json = embed_json[7..].to_string()
    }
    if embed_json.starts_with("```") {
        embed_json = embed_json[3..].to_string()
    }
    if embed_json.ends_with("```") {
        embed_json = embed_json[0 .. embed_json.len() - 3].to_string()
    }

    if !embed_json.ends_with("}") {
        embed_json += "}";
    }
    if !embed_json.starts_with("{") {
        embed_json = "{".to_string() + &embed_json;
    }

    let embed = serde_json::from_str::<Value>(&embed_json)?;
    let message = json!({ "embed" : embed });

    ctx.http.send_message(msg.channel_id.0, &message).await?;

    Ok(())
}

/// Sends the source code url to the bot.
#[command]
async fn source(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(ctx, "<https://gitlab.com/nitsuga5124/robo-arc/>").await?;
    Ok(())
}

/// Sends the current TO-DO list of the bot
#[command]
#[aliases(todo_list)]
async fn todo(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(ctx, "```prolog
TODO:

#Osu! 
MapPP (calculates pp of a map, like ezpp or tillerino) / don not use oppai.

#Twitch
ConfigureMessage (to configure the live message)

#Reddit / tsu_katana has to make this.
Subreddit (posts a random post from the subreddit specified)
User (posts a random post from the user specified)
Sub/User Bomb (posts 5 posts from the subreddit or user specified)

#Mod
Clear (add specific requieriments like \"only webhooks\") / waiting for the method to return a stream.
PermaBan (permanently bans a user from the guild by not allowing the user to ever get back on (perma kick))
TempMute (mutes the user on the specific channel or all channels)
Logging (set a channel to log specific events)
RuleManagement (https://5124.16-b.it/ss/22:50:34_21-06-2020.png)

#Tags
\"Basically the same as R. Danny, but with personal tags supported\"

# Boorus
\"Fix behoimi\"
Sauce (sends the source of an image, using iqdb and saucenao)
Exclude (excludes tags automatically from your search)
```").await?;
    Ok(())
}

/// Sends the current prefixes set to the server.
#[command]
#[aliases(prefixes)]
async fn prefix(ctx: &Context, msg: &Message) -> CommandResult {
    let data_read = ctx.data.read().await;
    let guild_id = &msg.guild_id;

    let prefix;
    if let Some(id) = guild_id {
        // obtain the id of the guild as an i64, because the id is stored as a u64, which is
        // not compatible with the postgre datbase types.
        let gid = id.0 as i64;

        // Obtain the database connection.
        let pool = data_read.get::<ConnectionPool>().unwrap();
        // Read the configured prefix of the guild from the database.
        let db_prefix = sqlx::query!("SELECT prefix FROM prefixes WHERE guild_id = $1", gid)
            .fetch(pool).boxed().try_next().await?;
        // If the guild doesn't have a configured prefix, return the default prefix.
        if let None = db_prefix {
            prefix = ".".to_string();
        // Else, just read the value that was stored on the database and return it.
        } else {
            prefix = db_prefix.unwrap().prefix.unwrap().to_string();
        }
    } else {
        prefix = ".".to_string();
    }

    msg.channel_id.say(ctx, format!("Current prefix:\n`{}`", &prefix)).await?;

    Ok(())
}

/// Sends information about the bot.
#[command]
#[aliases(info)]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let shard_latency = {
        let data = ctx.data.read().await;
        let shard_manager = data.get::<ShardManagerContainer>().unwrap();

        let manager = shard_manager.lock().await;
        let runners = manager.runners.lock().await;

        let runner_raw = runners.get(&ShardId(ctx.shard_id));
        if let Some(runner) = runner_raw {
            match runner.latency {
                Some(ms) => format!("{}ms", ms.as_millis()),
                _ => "?ms".to_string(),
            }
        } else {
            "?ms".to_string()
        }
    };

    let uptime = {
        let data = ctx.data.read().await;
        let instant = data.get::<Uptime>().unwrap();
        let duration = instant.elapsed();
        seconds_to_days(duration.as_secs())
    };

    let map = json!({"content" : "Calculating latency..."});

    let now = Instant::now();
    let mut message = ctx.http.send_message(msg.channel_id.0, &map).await?;
    let rest_latency = now.elapsed().as_millis();

    let pid = id().to_string();

    let full_stdout = Command::new("sh")
            .arg("-c")
            .arg(format!("./full_memory.sh {}", &pid).as_str())
            .output()
            .await
            .expect("failed to execute process");
    let reasonable_stdout = Command::new("sh")
            .arg("-c")
            .arg(format!("./reasonable_memory.sh {}", &pid).as_str())
            .output()
            .await
            .expect("failed to execute process");

    let mut full_mem = String::from_utf8(full_stdout.stdout).unwrap();
    let mut reasonable_mem = String::from_utf8(reasonable_stdout.stdout).unwrap();

    full_mem.pop();
    full_mem.pop();
    reasonable_mem.pop();
    reasonable_mem.pop();

    let version = {
        let mut file = File::open("Cargo.toml")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let data = contents.parse::<Value>().unwrap();
        let version = data["package"]["version"].as_str().unwrap();
        version.to_string()
    };

    let (hoster_tag, hoster_id) = {
        let app_info = ctx.http.get_current_application_info().await?;

        (app_info.owner.tag(), app_info.owner.id)
    };

    let current_user = ctx.cache.current_user().await;

    let bot_name = &current_user.name;
    let bot_icon = &current_user.avatar_url();

    let num_guilds = ctx.cache.guilds().await.len();
    let num_shards = ctx.cache.shard_count().await;
    let num_channels = ctx.cache.guild_channel_count().await;
    let num_priv_channels = ctx.cache.private_channels().await.len();

    let mut c_blank = 0;
    let mut c_comment = 0;
    let mut c_code = 0;
    let mut c_lines = 0;
    let mut command_count = 0;

    for entry in WalkDir::new("src") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let count = loc::count(path.to_str().unwrap());
            let text = read_to_string(&path)?;

            command_count += text.match_indices("#[command]").count();
            c_blank += count.blank;
            c_comment += count.comment;
            c_code += count.code;
            c_lines += count.lines;
        }
    }

    message.edit(ctx, |m| {
        m.content("");
        m.embed(|e| {
            e.title(format!("**{}** - v{}", bot_name, version));
            e.url("https://gitlab.com/nitsuga5124/robo-arc");
            e.description("General Purpose Discord Bot made in [Rust](https://www.rust-lang.org/) using [serenity.rs](https://github.com/serenity-rs/serenity)\n\nHaving any issues? join the [Support Server](https://discord.gg/kH7z85n)\nBe sure to `invite` me to your server if you like what i can do!");

            //e.field("Creator", "Tag: nitsuga5124#2207\nID: 182891574139682816", true);
            e.field("Statistics:", format!("Shards: {}\nGuilds: {}\nChannels: {}\nPrivate Channels: {}", num_shards, num_guilds, num_channels, num_priv_channels), true);
            e.field("Lines of code:", format!("Blank: {}\nComment: {}\nCode: {}\nTotal Lines: {}", c_blank, c_comment, c_code, c_lines), true);
            e.field("Currently hosted by:", format!("Tag: {}\nID: {}", hoster_tag, hoster_id), true);
            e.field("Latency:", format!("Gateway:\n`{}`\nREST:\n`{}ms`", shard_latency, rest_latency), true);
            e.field("Memory usage:", format!("Complete:\n`{} KB`\nBase:\n`{} KB`",
                                            &full_mem.parse::<u32>().expect("NaN").to_formatted_string(&Locale::en),
                                            &reasonable_mem.parse::<u32>().expect("NaN").to_formatted_string(&Locale::en)
                                            ), true);
            e.field("Somewhat Static Stats:", format!("Command Count:\n`{}`\nUptime:\n`{}`", command_count, uptime), true);

            if let Some(x) = bot_icon {
                e.thumbnail(x);
            }
            e
        });
        m
    }).await?;

    Ok(())
}

/// Sends the bot changelog.
#[command]
async fn changelog(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(ctx, "<https://gitlab.com/nitsuga5124/robo-arc/-/blob/master/CHANGELOG.md>").await?;
    Ok(())
}

#[command]
#[owners_only]
async fn reload_db(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    data.insert::<ConnectionPool>(obtain_pool().await?);
    msg.channel_id.say(ctx, "Ok.").await?;
    Ok(())
}
