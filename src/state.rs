use serde::Deserialize;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;

/// holds the value of a property
///
/// can be used in a deserializable type to accept any of the below
#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum PropValue {
    Boolean(bool),
    String(String),
    Int(i64),
    Uint(u64),
}

/// describes the type of a property and its default value
#[derive(Clone)]
enum PropType {
    Bool(bool),
    String(&'static str),
    /// default, min, max
    Int(i64, i64, i64),
    /// default, min, max
    Uint(u64, u64, u64),
    /// special value, indicates the type is a string, and its default is the output from the now() function
    Datetime,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PropAccess {
    None,
    Read,
    Write,
}

/// an iterator to list all saves in the saves folder
///
/// instanciate with `Save::iter()`
pub struct SaveIter(Option<std::fs::ReadDir>);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    NotFound,
    AlreadyExists,
    VersionNotFound,
    PropertyNotFound,
    IOError,
}

impl From<std::io::Error> for SaveError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            ErrorKind::AlreadyExists => Self::AlreadyExists,
            ErrorKind::NotFound => Self::NotFound,
            _ => Self::IOError,
        }
    }
}

pub mod save {
    use super::*;
    /// creates the save with the versions specified and returns the same as load would
    pub fn create(name: &str, version: &str) -> Result<String, SaveError> {
        if !std::fs::metadata(format!("versions/{version}.jar")).is_ok() {
            return Err(SaveError::VersionNotFound);
        }
        std::fs::create_dir(format!("saves/{name}"))?;
        let properties = generate_default_properties(version);
        if (|| {
            // folder created successfully, move files into the folder
            std::fs::write(
                format!("saves/{name}/eula.txt"),
                "# file auto created by mc-manager\r\neula=true\r\n",
            )?;
            std::fs::write(format!("saves/{name}/server.properties"), properties)?;
            std::fs::copy(
                format!("versions/{version}.jar"),
                format!("saves/{name}/server.jar"),
            )?;
            Ok::<(), std::io::Error>(())
        })()
        .is_err()
        {
            std::fs::remove_dir_all(format!("saves/{name}"))?;
            // the creation failed
            return Err(SaveError::IOError);
        }
        load(name)
    }
    /// delete the save specified and all backups
    pub fn delete(name: &str) -> Result<(), SaveError> {
        std::fs::remove_dir_all(format!("saves/{name}"))?;
        Ok(())
    }
    /// returns a valid json with all of the properties for a save, including its name
    pub fn load(name: &str) -> Result<String, SaveError> {
        folder_exists(format!("saves/{name}"))?;
        let properties = read_properties(format!("saves/{name}/server.properties"))?;
        let mut out = String::with_capacity(4096);
        out += "{\"name\":\"";
        out += name;
        out += "\"";
        for (access, name, ty, _) in PROPERTIES.iter() {
            if *access == PropAccess::None {
                continue;
            }
            out += ",\"";
            out += name;
            out += "\":";
            if let Some(value) = properties.get(*name) {
                match ty {
                    PropType::Bool(_) | PropType::Int(..) | PropType::Uint(..) => {
                        out += value;
                    }
                    PropType::String(_) | PropType::Datetime => {
                        append_json_string(&mut out, &value)
                    }
                }
            } else {
                out += "null";
            }
        }
        out += "}";
        Ok(out)
    }
    /// modifies one property of the save
    pub fn modify(name: &str, values: HashMap<String, PropValue>) -> Result<(), SaveError> {
        folder_exists(format!("saves/{name}"))?;
        validate_properties(&values, PropAccess::Write)?;
        write_properties(format!("saves/{name}/server.properties"), values)
    }
    /// update the access time of the world specified to now
    pub fn access(name: &str) -> Result<(), SaveError> {
        let mut values = HashMap::new();
        values.insert(
            "mc-manager-access-time".to_owned(),
            PropValue::String(now()),
        );
        write_properties(format!("saves/{name}/server.properties"), values)
    }
    /// iterate over the names of all saves avaiable
    pub fn iter() -> Result<SaveIter, SaveError> {
        match std::fs::read_dir("saves") {
            Ok(paths) => Ok(SaveIter(Some(paths))),
            Err(_) => Err(SaveError::IOError),
        }
    }
}

impl Iterator for SaveIter {
    type Item = Result<String, SaveError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.0 {
            match iter.next() {
                Some(Ok(path)) => {
                    if let Some(filename) = path.file_name().to_str() {
                        Some(Ok(filename.to_owned()))
                    } else {
                        self.next()
                    }
                }
                Some(Err(_)) => {
                    self.0 = None;
                    Some(Err(SaveError::IOError))
                }
                None => {
                    self.0 = None;
                    None
                }
            }
        } else {
            None
        }
    }
}

/// reads the only the ones especified properties, if not found returns None
fn read_properties(path: impl AsRef<Path>) -> Result<HashMap<String, String>, SaveError> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    let mut out = HashMap::new();

    let reader = BufReader::new(File::open(path)?);

    for line in reader.lines() {
        let line = line?;
        if !line.starts_with('#') {
            if let Some((key, value)) = line.split_once('=') {
                out.insert(key.trim().to_owned(), value.to_owned());
            }
        }
    }

    Ok(out)
}

/// writes the propertie to the file
fn write_properties(
    path: impl AsRef<Path> + Clone,
    mut values: HashMap<String, PropValue>,
) -> Result<(), SaveError> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    // the contents of the entire file
    let mut out = String::with_capacity(4 * 1024);

    let reader = BufReader::new(File::open(path.clone())?);

    for line in reader.lines() {
        let line = line?;
        if !line.starts_with('#') {
            if let Some((raw_key, _)) = line.split_once('=') {
                if let Some(new_value) = values.remove(raw_key.trim()) {
                    out += raw_key;
                    out += "=";
                    out += &new_value.to_value();
                    out += "\r\n";
                    continue;
                }
            }
        }
        out += &line;
        out += "\r\n";
    }

    for (key, value) in values {
        out += &key;
        out += "=";
        out += &value.to_value();
        out += "\r\n";
    }

    std::fs::write(path, out)?;

    Ok(())
}

fn validate_properties(
    values: &HashMap<String, PropValue>,
    access_needed: PropAccess,
) -> Result<(), SaveError> {
    for (key, value) in values.iter() {
        if let Some((access, ty)) = PROPERTIES
            .iter()
            .filter(|(_, name, _, _)| *name == key)
            .map(|(access, _, ty, _)| (*access, ty))
            .next()
        {
            match (access_needed, access) {
                (_, PropAccess::Write)
                | (PropAccess::None, _)
                | (PropAccess::Read, PropAccess::Read) => match ty {
                    PropType::Bool(_) => {
                        if let PropValue::Boolean(_) = value {
                            continue;
                        }
                    }
                    PropType::String(_) => {
                        if let PropValue::String(_) = value {
                            continue;
                        }
                    }
                    PropType::Int(_, min, max) => {
                        if let PropValue::Int(value) = value {
                            if value >= min && value <= max {
                                continue;
                            }
                        } else if let PropValue::Uint(value) = value {
                            if let Ok(value) = i64::try_from(*value) {
                                if value >= *min && value <= *max {
                                    continue;
                                }
                            }
                        }
                    }
                    PropType::Uint(_, min, max) => {
                        if let PropValue::Uint(value) = value {
                            if value >= min && value <= max {
                                continue;
                            }
                        } else if let PropValue::Int(value) = value {
                            if let Ok(value) = u64::try_from(*value) {
                                if value >= *min && value <= *max {
                                    continue;
                                }
                            }
                        }
                    }
                    PropType::Datetime => {
                        if let PropValue::String(value) = value {
                            if value.len() == 19 {
                                if let Ok(_) = chrono::NaiveDateTime::parse_from_str(
                                    value,
                                    "%Y-%m-%d %H:%M:%S",
                                ) {
                                    continue;
                                }
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        return Err(SaveError::PropertyNotFound);
    }
    Ok(())
}

fn folder_exists(path: impl AsRef<Path>) -> Result<(), SaveError> {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                Ok(())
            } else {
                Err(SaveError::NotFound)
            }
        }
        Err(error) => {
            const ERROR_FILE_NOT_FOUND: i32 = 2;
            const ERROR_PATH_NOT_FOUND: i32 = 3;
            if matches!(
                error.raw_os_error(),
                Some(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND)
            ) {
                Err(SaveError::NotFound)
            } else {
                Err(SaveError::IOError)
            }
        }
    }
}

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn append_json_string(out: &mut String, text: &str) {
    *out += "\"";
    for byte in text.bytes() {
        match byte {
            b'"' => *out += "\\\"",
            b'\\' => *out += "\\\\",
            7 => *out += "\\b",
            12 => *out += "\\f",
            b'\n' => *out += "\\n",
            b'\r' => *out += "\\r",
            b'\t' => *out += "\\t",
            0..=31 | 128..=255 => {
                *out += "\\x";
                let upper = byte >> 4;
                if upper < 10 {
                    out.push((b'0' + upper) as char);
                } else {
                    out.push((b'A' + upper - 10) as char);
                }
                let lower = byte & 0xF;
                if lower < 10 {
                    out.push((b'0' + lower) as char);
                } else {
                    out.push((b'A' + lower - 10) as char);
                }
            }
            _ => {
                out.push(byte as char);
            }
        }
    }
    *out += "\"";
}

impl PropValue {
    fn to_value(&self) -> String {
        match self {
            PropValue::Boolean(true) => "true".to_owned(),
            PropValue::Boolean(false) => "false".to_owned(),
            PropValue::String(value) => {
                let mut out = String::with_capacity(value.len() + 16);
                append_json_string(&mut out, value);
                out
            }
            PropValue::Int(value) => value.to_string(),
            PropValue::Uint(value) => value.to_string(),
        }
    }
}

fn generate_default_properties(version: &str) -> String {
    let mut out = String::new();
    let now = now();
    for (access, name, ty, _) in PROPERTIES.iter() {
        if *access == PropAccess::None {
            continue;
        }
        out += name;
        out += "=";
        if *name == "mc-manager-server-version" {
            out += version;
        } else {
            match ty {
                PropType::Bool(true) => out += "true",
                PropType::Bool(false) => out += "false",
                PropType::String(value) => out += value,
                PropType::Int(value, _, _) => out += &value.to_string(),
                PropType::Uint(value, _, _) => out += &value.to_string(),
                PropType::Datetime => out += &now,
            }
        }
        out += "\r\n";
    }
    out
}

const PROPERTIES: [(PropAccess, &str, PropType, &str); 61] = [
    (PropAccess::Write,"allow-flight",PropType::Bool(true),"Allows users to use flight on the server while in Survival mode, if they have a mod that provides flight installed. With allow-flight enabled, griefers may become more common, because it makes their work easier. In Creative mode, this has no effect. false - Flight is not allowed (players in air for at least 5 seconds get kicked). true - Flight is allowed, and used if the player has a fly mod installed."),
    (PropAccess::Write,"allow-nether",PropType::Bool(true),"Allows players to travel to the Nether . false - Nether portals do not work. true - The server allows portals to send players to the Nether."),
    (PropAccess::Write,"broadcast-console-to-ops",PropType::Bool(true),"Send console command outputs to all online operators ."),
    (PropAccess::Write,"broadcast-rcon-to-ops",PropType::Bool(true),"Send rcon console command outputs to all online operators."),
    (PropAccess::Write,"difficulty",PropType::Uint(3, 0, 3),"Defines the difficulty (such as damage dealt by mobs and the way hunger and poison affects players) of the server. If a legacy difficulty number is specified, it is silently converted to a difficulty name. peaceful (0) easy (1) normal (2) hard (3)"),
    (PropAccess::Write,"enable-command-block",PropType::Bool(true),"Enables command blocks"),
    (PropAccess::None,"enable-jmx-monitoring",PropType::Bool(false),"Exposes an MBean with the Object name net.minecraft.server:type=Server and two attributes averageTickTime and tickTimes exposing the tick times in milliseconds. In order for enabling JMX on the Java runtime you also need to add a couple of JVM flags to the startup as documented here ."),
    (PropAccess::None,"enable-rcon",PropType::Bool(false),"Enables remote access to the server console. It's not recommended to expose RCON to the Internet, because RCON protocol transfers everything without encryption. Everything (including RCON password) communicated between the RCON server and client can be leaked to someone listening in on your connection."),
    (PropAccess::Write,"enable-status",PropType::Bool(true),"Makes the server appear as \"online\" on the server list. If set to false, it will suppress replies from clients. This means it will appear as offline, but will still accept connections."),
    (PropAccess::None,"enable-query",PropType::Bool(false),"Enables GameSpy4 protocol server listener. Used to get information about server."),
    (PropAccess::Write,"enforce-secure-profile",PropType::Bool(true),"If set to true , players without a Mojang-signed public key will not be able to connect to the server."),
    (PropAccess::Write,"enforce-whitelist",PropType::Bool(false),"Enforces the whitelist on the server. When this option is enabled, users who are not present on the whitelist (if it's enabled) get kicked from the server after the server reloads the whitelist file. false - No user gets kicked if not on the whitelist. true - Online users not on the whitelist get kicked."),
    (PropAccess::Write,"entity-broadcast-range-percentage",PropType::Uint(100, 10, 1000),"Controls how close entities need to be before being sent to clients. Higher values means they'll be rendered from farther away, potentially causing more lag. This is expressed the percentage of the default value. For example, setting to 50 will make it half as usual. This mimics the function on the client video settings (not unlike Render Distance, which the client can customize so long as it's under the server's setting)."),
    (PropAccess::Write,"force-gamemode",PropType::Bool(false),"Force players to join in the default game mode . false - Players join in the gamemode they left in. true - Players always join in the default gamemode."),
    (PropAccess::Write,"function-permission-level",PropType::Uint(2, 1, 4),"Sets the default permission level for functions . See permission level for the details on the 4 levels."),
    (PropAccess::Write,"gamemode",PropType::Uint(0, 0, 3),"Defines the mode of gameplay . If a legacy gamemode number is specified, it is silently converted to a gamemode name. survival (0) creative (1) adventure (2) spectator (3)"),
    (PropAccess::Write,"generate-structures",PropType::Bool(true),"Defines whether structures (such as villages) can be generated. false - Structures are not generated in new chunks. true - Structures are generated in new chunks. Note: Dungeons still generate if this is set to false."),
    (PropAccess::Write,"generator-settings",PropType::String("{}"),"The settings used to customize world generation. Follow its format and write the corresponding JSON string. Remember to escape all : with \\: ."),
    (PropAccess::Write,"hardcore",PropType::Bool(false),"If set to true , server difficulty is ignored and set to hard and players are set to spectator mode if they die."),
    (PropAccess::Write,"hide-online-players",PropType::Bool(false),"If set to true , a player list is not sent on status requests."),
    (PropAccess::Write,"initial-disabled-packs",PropType::String(""),"Comma-separated list of datapacks to not be auto-enabled on world creation."),
    (PropAccess::Write,"initial-enabled-packs",PropType::String("vanilla"),"Comma-separated list of datapacks to be enabled during world creation. Feature packs need to be explicitly enabled."),
    (PropAccess::None,"level-name",PropType::String("world"),"The \"level-name\" value is used as the world name and its folder name. The player may also copy their saved game folder here, and change the name to the same as that folder's to load it instead. Characters such as ' (apostrophe) may need to be escaped by adding a backslash before them."),
    (PropAccess::Write,"level-seed",PropType::String(""),"Sets a world seed for the player's world, as in Singleplayer. The world generates with a random seed if left blank. Some examples are: minecraft, 404, 1a2b3c."),
    (PropAccess::Write,"level-type",PropType::String("minecraft:normal"),"Determines the world preset that is generated. Escaping \":\" is required when using a world preset ID, and the vanilla world preset ID's namespace ( minecraft: ) can be omitted. minecraft:normal - Standard world with hills, valleys, water, etc. minecraft: flat - A flat world with no features, can be modified with generator-settings . minecraft: large_biomes - Same as default but all biomes are larger. minecraft: amplified - Same as default but world-generation height limit is increased. minecraft: single_biome_surface - A buffet world which the entire overworld consists of one biome, can be modified with generator-settings . buffet - Only for 1.15 or before. Same as default unless generator-settings is set. default_1_1 - Only for 1.15 or before. Same as default, but counted as a different world type. customized - Only for 1.15 or before. After 1.13, this value is no different than default, but in 1.12 and before, it could be used to create a completely custom world."),
    (PropAccess::Write,"max-chained-neighbor-updates",PropType::Int(1000000, -1, i64::MAX),"Limiting the amount of consecutive neighbor updates before skipping additional ones. Negative values remove the limit."),
    (PropAccess::Write,"max-players",PropType::Uint(20, 0, u32::MAX as u64),"The maximum number of players that can play on the server at the same time. Note that more players on the server consume more resources. Note also, op player connections are not supposed to count against the max players, but ops currently cannot join a full server. However, this can be changed by going to the file called ops.json in the player's server directory, opening it, finding the op that the player wants to change, and changing the setting called bypassesPlayerLimit to true (the default is false). This means that that op does not have to wait for a player to leave in order to join. Extremely large values for this field result in the client-side user list being broken."),
    (PropAccess::Write,"max-tick-time",PropType::Uint(60000, 0, u64::MAX),"The maximum number of milliseconds a single tick may take before the server watchdog stops the server with the message, A single server tick took 60.00 seconds (should be max 0.05); Considering it to be crashed, server will forcibly shutdown. Once this criterion is met, it calls System.exit(1). -1 - disable watchdog entirely (this disable option was added in 14w32a)"),
    (PropAccess::Write,"max-world-size",PropType::Uint(60000, 0, 29999984),"This sets the maximum possible size in blocks, expressed as a radius, that the world border can obtain. Setting the world border bigger causes the commands to complete successfully but the actual border does not move past this block limit. Setting the max-world-size higher than the default doesn't appear to do anything. Examples: Setting max-world-size to 1000 allows the player to have a 2000x2000 world border. Setting max-world-size to 4000 gives the player an 8000x8000 world border."),
    (PropAccess::Read,"mc-manager-server-version",PropType::String(""),"A variable for mc-manager, to keep track of what server version this is."),
    (PropAccess::Read,"mc-manager-create-time",PropType::Datetime,"A variable for mc-manager, to keep track when this save was created."),
    (PropAccess::Read,"mc-manager-access-time",PropType::Datetime,"A variable for mc-manager, to keep track when this save was last online."),
    (PropAccess::Write,"motd",PropType::String("A Minecraft Server"),"This is the message that is displayed in the server list of the client, below the name. The MOTD supports color and formatting codes . The MOTD supports special characters, such as \"♥\". However, such characters must be converted to escaped Unicode form. An online converter can be found here . If the MOTD is over 59 characters, the server list may report a communication error."),
    (PropAccess::Write,"network-compression-threshold",PropType::Uint(256, 0, u64::MAX),"By default it allows packets that are n-1 bytes big to go normally, but a packet of n bytes or more gets compressed down. So, a lower number means more compression but compressing small amounts of bytes might actually end up with a larger result than what went in. -1 - disable compression entirely 0 - compress everything Note: The Ethernet spec requires that packets less than 64 bytes become padded to 64 bytes. Thus, setting a value lower than 64 may not be beneficial. It is also not recommended to exceed the MTU, typically 1500 bytes."),
    (PropAccess::Write,"online-mode",PropType::Bool(true),"Server checks connecting players against Minecraft account database. Set this to false only if the player's server is not connected to the Internet. Hackers with fake accounts can connect if this is set to false! If minecraft.net is down or inaccessible, no players can connect if this is set to true. Setting this variable to off purposely is called \"cracking\" a server, and servers that are present with online mode off are called \"cracked\" servers, allowing players with unlicensed copies of Minecraft to join. true - Enabled. The server assumes it has an Internet connection and checks every connecting player. false - Disabled. The server does not attempt to check connecting players."),
    (PropAccess::Write,"op-permission-level",PropType::Uint(4, 0, 4),"Sets the default permission level for ops when using / op ."),
    (PropAccess::Write,"player-idle-timeout",PropType::Uint(0, 0, u64::MAX),"If non-zero, players are kicked from the server if they are idle for more than that many minutes. Note: Idle time is reset when the server receives one of the following packets: Click Window Enchant Item Update Sign Player Digging Player Block Placement Held Item Change Animation (swing arm) Entity Action Client Status Chat Message Use Entity"),
    (PropAccess::Write,"prevent-proxy-connections",PropType::Bool(false),"If the ISP/AS sent from the server is different from the one from Mojang Studios' authentication server, the player is kicked."),
    (PropAccess::Write,"previews-chat",PropType::Bool(false),"If set to true , chat preview will be enabled. true - Enabled. When enabled, a server-controlled preview appears above the chat edit box, showing how the message will look when sent. false - Disabled."),
    (PropAccess::Write,"pvp",PropType::Bool(true),"Enable PvP on the server. Players shooting themselves with arrows receive damage only if PvP is enabled. true - Players can kill each other. false - Players cannot kill other players (also known as Player versus Environment ( PvE )). Note: Indirect damage sources spawned by players (such as lava , fire , TNT and to some extent water , sand and gravel ) still deal damage to other players."),
    (PropAccess::Write,"query.port",PropType::Uint(25565, 1, u16::MAX as u64),"Sets the port for the query server (see enable-query )."),
    (PropAccess::Write,"rate-limit",PropType::Uint(0, 0, u64::MAX),"Sets the maximum amount of packets a user can send before getting kicked. Setting to 0 disables this feature."),
    (PropAccess::None,"rcon.password",PropType::String(""),"Sets the password for RCON: a remote console protocol that can allow other applications to connect and interact with a Minecraft server over the internet."),
    (PropAccess::None,"rcon.port",PropType::Uint(25575, 1, u16::MAX as u64),"Sets the RCON network port."),
    (PropAccess::Write,"resource-pack",PropType::String(""),"Optional URI to a resource pack . The player may choose to use it. Note that (in some versions before 1.15.2), the \":\" and \"=\" characters need to be escaped with a backslash (\\), e.g. http\\://somedomain.com/somepack.zip?someparam\\=somevalue The resource pack may not have a larger file size than 250 MiB (Before 1.18: 100 MiB (≈ 100.8 MB)) (Before 1.15: 50 MiB (≈ 50.4 MB)). Note that download success or failure is logged by the client, and not by the server."),
    (PropAccess::Write,"resource-pack-prompt",PropType::String(""),"Optional, adds a custom message to be shown on resource pack prompt when require-resource-pack is used. Expects chat component syntax, can contain multiple lines."),
    (PropAccess::Write,"resource-pack-sha1",PropType::String(""),"Optional SHA-1 digest of the resource pack, in lowercase hexadecimal. It is recommended to specify this, because it is used to verify the integrity of the resource pack. Note: If the resource pack is any different, a yellow message \"Invalid sha1 for resource-pack-sha1\" appears in the console when the server starts. Due to the nature of hash functions, errors have a tiny probability of occurring, so this consequence has no effect."),
    (PropAccess::Write,"require-resource-pack",PropType::Bool(false),"When this option is enabled (set to true), players will be prompted for a response and will be disconnected if they decline the required pack."),
    (PropAccess::Write,"server-ip",PropType::String(""),"The player should set this if they want the server to bind to a particular IP. It is strongly recommended that the player leaves server-ip blank. Set to blank, or the IP the player want their server to run (listen) on."),
    (PropAccess::Write,"server-port",PropType::Uint(25565, 1, u16::MAX as u64),"Changes the port the server is hosting (listening) on. This port must be forwarded if the server is hosted in a network using NAT (if the player has a home router/firewall)."),
    (PropAccess::Write,"simulation-distance",PropType::Uint(10, 3, 32),"Sets the maximum distance from players that living entities may be located in order to be updated by the server, measured in chunks in each direction of the player (radius, not diameter). If entities are outside of this radius, then they will not be ticked by the server nor will they be visible to players. 10 is the default/recommended. If the player has major lag, this value is recommended to be reduced."),
    (PropAccess::None,"snooper-enabled",PropType::Bool(false),"Sets whether the server sends snoop data regularly to http://snoop.minecraft.net . false - disable snooping. true - enable snooping."),
    (PropAccess::Write,"spawn-animals",PropType::Bool(true),"Determines if animals can spawn. true - Animals spawn as normal. false - Animals immediately vanish. If the player has major lag, it is recommended to turn this off/set to false."),
    (PropAccess::Write,"spawn-monsters",PropType::Bool(true),"Determines if monsters can spawn. true - Enabled. Monsters appear at night and in the dark. false - Disabled. No monsters. This setting has no effect if difficulty = 0 (peaceful). If difficulty is not = 0, a monster can still spawn from a monster spawner . If the player has major lag, it is recommended to turn this off/set to false."),
    (PropAccess::Write,"spawn-npcs",PropType::Bool(true),"Determines whether villagers can spawn. true - Enabled. Villagers spawn. false - Disabled. No villagers."),
    (PropAccess::Write,"spawn-protection",PropType::Uint(0, 0, u64::MAX),"Determines the side length of the square spawn protection area as 2 x +1. Setting this to 0 disables the spawn protection. A value of 1 protects a 3x3 square centered on the spawn point. 2 protects 5x5, 3 protects 7x7, etc. This option is not generated on the first server start and appears when the first player joins. If there are no ops set on the server, the spawn protection is disabled automatically as well."),
    (PropAccess::Write,"sync-chunk-writes",PropType::Bool(true),"Enables synchronous chunk writes."),
    (PropAccess::None,"text-filtering-config",PropType::String(""),"[ more information needed ]"),
    (PropAccess::Write,"use-native-transport",PropType::Bool(true),"Linux server performance improvements: optimized packet sending/receiving on Linux true - Enabled. Enable Linux packet sending/receiving optimization false - Disabled. Disable Linux packet sending/receiving optimization"),
    (PropAccess::Write,"view-distance",PropType::Uint(10, 3, 32),"Sets the amount of world data the server sends the client, measured in chunks in each direction of the player (radius, not diameter). It determines the server-side viewing distance. 10 is the default/recommended. If the player has major lag, this value is recommended to be reduced."),
    (PropAccess::Write,"white-list",PropType::Bool(false),"Enables a whitelist on the server. With a whitelist enabled, users not on the whitelist cannot connect. Intended for private servers, such as those for real-life friends or strangers carefully selected via an application process, for example. false - No white list is used. true - The file whitelist.json is used to generate the white list. Note: Ops are automatically whitelisted, and there is no need to add them to the whitelist."),
];
