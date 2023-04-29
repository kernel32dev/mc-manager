use serde::Deserialize;

use crate::utils::append_prop_escaped;

/// defines a property completely
pub struct PropDef {
    pub access: PropAccess,
    pub ty: PropType,
    pub name: &'static str,
    pub label: &'static str,
    pub desc: &'static str,
}

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
pub enum PropType {
    Bool(bool),
    String(&'static str),
    /// default, min, max
    Int(i64, i64, i64),
    /// default, min, max
    Uint(u64, u64, u64),
    /// special value, indicates the type is a string, and its default is the output from the now() function
    Datetime,
    /// numeric enum, first value is the default, the string in each item of the array is the label, its index is the value
    #[allow(dead_code)]
    IntEnum(u64, &'static [&'static str]),
    /// string enum, first value is the index of the default pair, the second is an array of pairs, first string in each pair is the value, second string in each pair is the label
    StrEnum(usize, &'static [(&'static str, &'static str)]),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PropAccess {
    None,
    Read,
    Write,
}

impl PropValue {
    pub fn to_prop_value(&self, out: &mut String) {
        match self {
            PropValue::Boolean(true) => *out += "true",
            PropValue::Boolean(false) => *out += "false",
            PropValue::String(value) => append_prop_escaped(out, value),
            PropValue::Int(value) => *out += &value.to_string(),
            PropValue::Uint(value) => *out += &value.to_string(),
        }
    }
}

pub const CREATE_PROPERTIES: &[&'static str] = &[
    "motd",
    "level-seed",
    "gamemode",
    "difficulty",
    "server-port",
    "pvp",
    "max-players",
    "enable-command-block",
    "online-mode",
    "enforce-secure-profile",
    "level-type",
];

// TODO: translate and simplify label and desc

pub const PROPERTIES: &[PropDef] = &[
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "allow-flight",
        label: "allow-flight",
        desc: "Allows users to use flight on the server while in Survival mode, if they have a mod that provides flight installed. With allow-flight enabled, griefers may become more common, because it makes their work easier. In Creative mode, this has no effect. false - Flight is not allowed (players in air for at least 5 seconds get kicked). true - Flight is allowed, and used if the player has a fly mod installed.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "allow-nether",
        label: "allow-nether",
        desc: "Allows players to travel to the Nether . false - Nether portals do not work. true - The server allows portals to send players to the Nether.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "broadcast-console-to-ops",
        label: "broadcast-console-to-ops",
        desc: "Send console command outputs to all online operators .",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "broadcast-rcon-to-ops",
        label: "broadcast-rcon-to-ops",
        desc: "Send rcon console command outputs to all online operators.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::StrEnum(1, &[("peaceful", "Pacífico"), ("easy", "Fácil"), ("medium", "Médio"), ("hard", "Difícil")]),
        name: "difficulty",
        label: "difficulty",
        desc: "Defines the difficulty (such as damage dealt by mobs and the way hunger and poison affects players) of the server. If a legacy difficulty number is specified, it is silently converted to a difficulty name. peaceful (0) easy (1) normal (2) hard (3)",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "enable-command-block",
        label: "enable-command-block",
        desc: "Enables command blocks",
    },
    PropDef {
        access: PropAccess::None,
        ty: PropType::Bool(false),
        name: "enable-jmx-monitoring",
        label: "enable-jmx-monitoring",
        desc: "Exposes an MBean with the Object name net.minecraft.server:type=Server and two attributes averageTickTime and tickTimes exposing the tick times in milliseconds. In order for enabling JMX on the Java runtime you also need to add a couple of JVM flags to the startup as documented here .",
    },
    PropDef {
        access: PropAccess::None,
        ty: PropType::Bool(false),
        name: "enable-rcon",
        label: "enable-rcon",
        desc: "Enables remote access to the server console. It's not recommended to expose RCON to the Internet, because RCON protocol transfers everything without encryption. Everything (including RCON password) communicated between the RCON server and client can be leaked to someone listening in on your connection.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "enable-status",
        label: "enable-status",
        desc: "Makes the server appear as \"online\" on the server list. If set to false, it will suppress replies from clients. This means it will appear as offline, but will still accept connections.",
    },
    PropDef {
        access: PropAccess::None,
        ty: PropType::Bool(false),
        name: "enable-query",
        label: "enable-query",
        desc: "Enables GameSpy4 protocol server listener. Used to get information about server.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "enforce-secure-profile",
        label: "enforce-secure-profile",
        desc: "If set to true , players without a Mojang-signed public key will not be able to connect to the server.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(false),
        name: "enforce-whitelist",
        label: "enforce-whitelist",
        desc: "Enforces the whitelist on the server. When this option is enabled, users who are not present on the whitelist (if it's enabled) get kicked from the server after the server reloads the whitelist file. false - No user gets kicked if not on the whitelist. true - Online users not on the whitelist get kicked.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(100, 10, 1000),
        name: "entity-broadcast-range-percentage",
        label: "entity-broadcast-range-percentage",
        desc: "Controls how close entities need to be before being sent to clients. Higher values means they'll be rendered from farther away, potentially causing more lag. This is expressed the percentage of the default value. For example, setting to 50 will make it half as usual. This mimics the function on the client video settings (not unlike Render Distance, which the client can customize so long as it's under the server's setting).",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(false),
        name: "force-gamemode",
        label: "force-gamemode",
        desc: "Force players to join in the default game mode . false - Players join in the gamemode they left in. true - Players always join in the default gamemode.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(4, 1, 4),
        name: "function-permission-level",
        label: "function-permission-level",
        desc: "Sets the default permission level for functions . See permission level for the details on the 4 levels.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::StrEnum(0, &[("survival", "Modo Sobrevivência"), ("creative", "Modo Criativo"), ("adventure", "Modo Aventura"), ("spectator", "Modo Spectador")]),
        name: "gamemode",
        label: "gamemode",
        desc: "Defines the mode of gameplay . If a legacy gamemode number is specified, it is silently converted to a gamemode name. survival (0) creative (1) adventure (2) spectator (3)",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "generate-structures",
        label: "generate-structures",
        desc: "Defines whether structures (such as villages) can be generated. false - Structures are not generated in new chunks. true - Structures are generated in new chunks. Note: Dungeons still generate if this is set to false.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::String(""),
        name: "generator-settings",
        label: "generator-settings",
        desc: "The settings used to customize world generation. Follow its format and write the corresponding JSON string. Remember to escape all : with \\: .",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(false),
        name: "hardcore",
        label: "hardcore",
        desc: "If set to true , server difficulty is ignored and set to hard and players are set to spectator mode if they die.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(false),
        name: "hide-online-players",
        label: "hide-online-players",
        desc: "If set to true , a player list is not sent on status requests.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::String(""),
        name: "initial-disabled-packs",
        label: "initial-disabled-packs",
        desc: "Comma-separated list of datapacks to not be auto-enabled on world creation.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::String("vanilla"),
        name: "initial-enabled-packs",
        label: "initial-enabled-packs",
        desc: "Comma-separated list of datapacks to be enabled during world creation. Feature packs need to be explicitly enabled.",
    },
    PropDef {
        access: PropAccess::None,
        ty: PropType::String("world"),
        name: "level-name",
        label: "level-name",
        desc: "The \"level-name\" value is used as the world name and its folder name. The player may also copy their saved game folder here, and change the name to the same as that folder's to load it instead. Characters such as ' (apostrophe) may need to be escaped by adding a backslash before them.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::String(""),
        name: "level-seed",
        label: "level-seed",
        desc: "Sets a world seed for the player's world, as in Singleplayer. The world generates with a random seed if left blank. Some examples are: minecraft, 404, 1a2b3c.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::StrEnum(0, &[("normal","Normal"), ("flat","Plano"), ("large_biomes","Grandes Biomas"), ("amplified","Aplificado")]),
        name: "level-type",
        label: "level-type",
        desc: "Determines the world preset that is generated. Escaping \":\" is required when using a world preset ID, and the vanilla world preset ID's namespace ( minecraft: ) can be omitted. minecraft:normal - Standard world with hills, valleys, water, etc. minecraft: flat - A flat world with no features, can be modified with generator-settings . minecraft: large_biomes - Same as default but all biomes are larger. minecraft: amplified - Same as default but world-generation height limit is increased. minecraft: single_biome_surface - A buffet world which the entire overworld consists of one biome, can be modified with generator-settings . buffet - Only for 1.15 or before. Same as default unless generator-settings is set. default_1_1 - Only for 1.15 or before. Same as default, but counted as a different world type. customized - Only for 1.15 or before. After 1.13, this value is no different than default, but in 1.12 and before, it could be used to create a completely custom world.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Int(1000000, -1, i64::MAX),
        name: "max-chained-neighbor-updates",
        label: "max-chained-neighbor-updates",
        desc: "Limiting the amount of consecutive neighbor updates before skipping additional ones. Negative values remove the limit.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(20, 0, u32::MAX as u64),
        name: "max-players",
        label: "max-players",
        desc: "The maximum number of players that can play on the server at the same time. Note that more players on the server consume more resources. Note also, op player connections are not supposed to count against the max players, but ops currently cannot join a full server. However, this can be changed by going to the file called ops.json in the player's server directory, opening it, finding the op that the player wants to change, and changing the setting called bypassesPlayerLimit to true (the default is false). This means that that op does not have to wait for a player to leave in order to join. Extremely large values for this field result in the client-side user list being broken.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(60000, 0, u64::MAX),
        name: "max-tick-time",
        label: "max-tick-time",
        desc: "The maximum number of milliseconds a single tick may take before the server watchdog stops the server with the message, A single server tick took 60.00 seconds (should be max 0.05); Considering it to be crashed, server will forcibly shutdown. Once this criterion is met, it calls System.exit(1). -1 - disable watchdog entirely (this disable option was added in 14w32a)",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(60000, 0, 29999984),
        name: "max-world-size",
        label: "max-world-size",
        desc: "This sets the maximum possible size in blocks, expressed as a radius, that the world border can obtain. Setting the world border bigger causes the commands to complete successfully but the actual border does not move past this block limit. Setting the max-world-size higher than the default doesn't appear to do anything. Examples: Setting max-world-size to 1000 allows the player to have a 2000x2000 world border. Setting max-world-size to 4000 gives the player an 8000x8000 world border.",
    },
    PropDef {
        access: PropAccess::Read,
        ty: PropType::String(""),
        name: "mc-manager-server-version",
        label: "mc-manager-server-version",
        desc: "A variable for mc-manager, to keep track of what server version this is.",
    },
    PropDef {
        access: PropAccess::Read,
        ty: PropType::Datetime,
        name: "mc-manager-create-time",
        label: "mc-manager-create-time",
        desc: "A variable for mc-manager, to keep track when this save was created.",
    },
    PropDef {
        access: PropAccess::Read,
        ty: PropType::Datetime,
        name: "mc-manager-access-time",
        label: "mc-manager-access-time",
        desc: "A variable for mc-manager, to keep track when this save was last online.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::String("Um servidor de minecraft, gerenciando pelo mc-manager"),
        name: "motd",
        label: "motd",
        desc: "This is the message that is displayed in the server list of the client, below the name. The MOTD supports color and formatting codes . The MOTD supports special characters, such as \"♥\". However, such characters must be converted to escaped Unicode form. An online converter can be found here . If the MOTD is over 59 characters, the server list may report a communication error.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(256, 0, u64::MAX),
        name: "network-compression-threshold",
        label: "network-compression-threshold",
        desc: "By default it allows packets that are n-1 bytes big to go normally, but a packet of n bytes or more gets compressed down. So, a lower number means more compression but compressing small amounts of bytes might actually end up with a larger result than what went in. -1 - disable compression entirely 0 - compress everything Note: The Ethernet spec requires that packets less than 64 bytes become padded to 64 bytes. Thus, setting a value lower than 64 may not be beneficial. It is also not recommended to exceed the MTU, typically 1500 bytes.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "online-mode",
        label: "online-mode",
        desc: "Server checks connecting players against Minecraft account database. Set this to false only if the player's server is not connected to the Internet. Hackers with fake accounts can connect if this is set to false! If minecraft.net is down or inaccessible, no players can connect if this is set to true. Setting this variable to off purposely is called \"cracking\" a server, and servers that are present with online mode off are called \"cracked\" servers, allowing players with unlicensed copies of Minecraft to join. true - Enabled. The server assumes it has an Internet connection and checks every connecting player. false - Disabled. The server does not attempt to check connecting players.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(4, 0, 4),
        name: "op-permission-level",
        label: "op-permission-level",
        desc: "Sets the default permission level for ops when using / op .",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(0, 0, u64::MAX),
        name: "player-idle-timeout",
        label: "player-idle-timeout",
        desc: "If non-zero, players are kicked from the server if they are idle for more than that many minutes. Note: Idle time is reset when the server receives one of the following packets: Click Window Enchant Item Update Sign Player Digging Player Block Placement Held Item Change Animation (swing arm) Entity Action Client Status Chat Message Use Entity",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(false),
        name: "prevent-proxy-connections",
        label: "prevent-proxy-connections",
        desc: "If the ISP/AS sent from the server is different from the one from Mojang Studios' authentication server, the player is kicked.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(false),
        name: "previews-chat",
        label: "previews-chat",
        desc: "If set to true , chat preview will be enabled. true - Enabled. When enabled, a server-controlled preview appears above the chat edit box, showing how the message will look when sent. false - Disabled.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "pvp",
        label: "pvp",
        desc: "Enable PvP on the server. Players shooting themselves with arrows receive damage only if PvP is enabled. true - Players can kill each other. false - Players cannot kill other players (also known as Player versus Environment ( PvE )). Note: Indirect damage sources spawned by players (such as lava , fire , TNT and to some extent water , sand and gravel ) still deal damage to other players.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(25565, 1, u16::MAX as u64),
        name: "query.port",
        label: "query.port",
        desc: "Sets the port for the query server (see enable-query ).",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(0, 0, u64::MAX),
        name: "rate-limit",
        label: "rate-limit",
        desc: "Sets the maximum amount of packets a user can send before getting kicked. Setting to 0 disables this feature.",
    },
    PropDef {
        access: PropAccess::None,
        ty: PropType::String(""),
        name: "rcon.password",
        label: "rcon.password",
        desc: "Sets the password for RCON: a remote console protocol that can allow other applications to connect and interact with a Minecraft server over the internet.",
    },
    PropDef {
        access: PropAccess::None,
        ty: PropType::Uint(25575, 1, u16::MAX as u64),
        name: "rcon.port",
        label: "rcon.port",
        desc: "Sets the RCON network port.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::String(""),
        name: "resource-pack",
        label: "resource-pack",
        desc: "Optional URI to a resource pack . The player may choose to use it. Note that (in some versions before 1.15.2), the \":\" and \"=\" characters need to be escaped with a backslash (\\), e.g. http\\://somedomain.com/somepack.zip?someparam\\=somevalue The resource pack may not have a larger file size than 250 MiB (Before 1.18: 100 MiB (≈ 100.8 MB)) (Before 1.15: 50 MiB (≈ 50.4 MB)). Note that download success or failure is logged by the client, and not by the server.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::String(""),
        name: "resource-pack-prompt",
        label: "resource-pack-prompt",
        desc: "Optional, adds a custom message to be shown on resource pack prompt when require-resource-pack is used. Expects chat component syntax, can contain multiple lines.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::String(""),
        name: "resource-pack-sha1",
        label: "resource-pack-sha1",
        desc: "Optional SHA-1 digest of the resource pack, in lowercase hexadecimal. It is recommended to specify this, because it is used to verify the integrity of the resource pack. Note: If the resource pack is any different, a yellow message \"Invalid sha1 for resource-pack-sha1\" appears in the console when the server starts. Due to the nature of hash functions, errors have a tiny probability of occurring, so this consequence has no effect.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(false),
        name: "require-resource-pack",
        label: "require-resource-pack",
        desc: "When this option is enabled (set to true), players will be prompted for a response and will be disconnected if they decline the required pack.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::String(""),
        name: "server-ip",
        label: "server-ip",
        desc: "The player should set this if they want the server to bind to a particular IP. It is strongly recommended that the player leaves server-ip blank. Set to blank, or the IP the player want their server to run (listen) on.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(25565, 1, u16::MAX as u64),
        name: "server-port",
        label: "server-port",
        desc: "Changes the port the server is hosting (listening) on. This port must be forwarded if the server is hosted in a network using NAT (if the player has a home router/firewall).",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(10, 3, 32),
        name: "simulation-distance",
        label: "simulation-distance",
        desc: "Sets the maximum distance from players that living entities may be located in order to be updated by the server, measured in chunks in each direction of the player (radius, not diameter). If entities are outside of this radius, then they will not be ticked by the server nor will they be visible to players. 10 is the default/recommended. If the player has major lag, this value is recommended to be reduced.",
    },
    PropDef {
        access: PropAccess::None,
        ty: PropType::Bool(false),
        name: "snooper-enabled",
        label: "snooper-enabled",
        desc: "Sets whether the server sends snoop data regularly to http://snoop.minecraft.net . false - disable snooping. true - enable snooping.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "spawn-animals",
        label: "spawn-animals",
        desc: "Determines if animals can spawn. true - Animals spawn as normal. false - Animals immediately vanish. If the player has major lag, it is recommended to turn this off/set to false.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "spawn-monsters",
        label: "spawn-monsters",
        desc: "Determines if monsters can spawn. true - Enabled. Monsters appear at night and in the dark. false - Disabled. No monsters. This setting has no effect if difficulty = 0 (peaceful). If difficulty is not = 0, a monster can still spawn from a monster spawner . If the player has major lag, it is recommended to turn this off/set to false.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "spawn-npcs",
        label: "spawn-npcs",
        desc: "Determines whether villagers can spawn. true - Enabled. Villagers spawn. false - Disabled. No villagers.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(0, 0, u64::MAX),
        name: "spawn-protection",
        label: "spawn-protection",
        desc: "Determines the side length of the square spawn protection area as 2 x +1. Setting this to 0 disables the spawn protection. A value of 1 protects a 3x3 square centered on the spawn point. 2 protects 5x5, 3 protects 7x7, etc. This option is not generated on the first server start and appears when the first player joins. If there are no ops set on the server, the spawn protection is disabled automatically as well.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "sync-chunk-writes",
        label: "sync-chunk-writes",
        desc: "Enables synchronous chunk writes.",
    },
    PropDef {
        access: PropAccess::None,
        ty: PropType::String(""),
        name: "text-filtering-config",
        label: "text-filtering-config",
        desc: "[ more information needed ]",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(true),
        name: "use-native-transport",
        label: "use-native-transport",
        desc: "Linux server performance improvements: optimized packet sending/receiving on Linux true - Enabled. Enable Linux packet sending/receiving optimization false - Disabled. Disable Linux packet sending/receiving optimization",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Uint(10, 3, 32),
        name: "view-distance",
        label: "view-distance",
        desc: "Sets the amount of world data the server sends the client, measured in chunks in each direction of the player (radius, not diameter). It determines the server-side viewing distance. 10 is the default/recommended. If the player has major lag, this value is recommended to be reduced.",
    },
    PropDef {
        access: PropAccess::Write,
        ty: PropType::Bool(false),
        name: "white-list",
        label: "white-list",
        desc: "Enables a whitelist on the server. With a whitelist enabled, users not on the whitelist cannot connect. Intended for private servers, such as those for real-life friends or strangers carefully selected via an application process, for example. false - No white list is used. true - The file whitelist.json is used to generate the white list. Note: Ops are automatically whitelisted, and there is no need to add them to the whitelist.",
    },
];
