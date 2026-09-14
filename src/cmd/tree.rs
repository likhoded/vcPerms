use pumpkin_plugin_api::command::{
    Arg, ArgumentType, Command, CommandNode, CommandSender, CommandSuggestion, CommandSuggestions,
    ConsumedArgs, StringType, SuggestionRequest,
};
use pumpkin_plugin_api::commands::{CommandHandler, CommandSuggestionHandler};
use pumpkin_plugin_api::Server;

use crate::state::with_store;

use super::{run, Suggest};

#[derive(Clone)]
enum Piece {
    Lit(&'static str),
    Arg(&'static str),
}

struct Rebuild(Vec<Piece>);

impl CommandHandler for Rebuild {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, pumpkin_plugin_api::command::CommandError> {
        let mut bits = Vec::new();
        for piece in &self.0 {
            match piece {
                Piece::Lit(s) => bits.push((*s).to_string()),
                Piece::Arg(key) => {
                    if let Some(v) = arg_text(&args, key) {
                        if !v.is_empty() {
                            bits.push(v);
                        }
                    }
                }
            }
        }
        run(&sender, &server, &bits.join(" "))
    }
}

fn arg_text(args: &ConsumedArgs, key: &str) -> Option<String> {
    match args.get_value(key) {
        Arg::Simple(s) | Arg::Msg(s) => Some(s),
        Arg::Bool(b) => Some(if b { "true".into() } else { "false".into() }),
        _ => None,
    }
}

fn word() -> ArgumentType {
    ArgumentType::String(StringType::SingleWord)
}

fn greedy() -> ArgumentType {
    ArgumentType::String(StringType::Greedy)
}

fn lit(name: &str) -> CommandNode {
    CommandNode::literal(name)
}

fn arg(name: &str, ty: ArgumentType) -> CommandNode {
    CommandNode::argument(name, &ty)
}

fn rb(pieces: &[Piece]) -> Rebuild {
    Rebuild(pieces.to_vec())
}

fn l(s: &'static str) -> Piece {
    Piece::Lit(s)
}

fn a(s: &'static str) -> Piece {
    Piece::Arg(s)
}

pub fn attach(mut root: Command) -> Command {
    for name in [
        "help", "info", "reload", "sync", "networksync", "editor", "listgroups", "listtracks",
    ] {
        root = root.then(lit(name).execute(rb(&[l(name)])));
    }

    root = root.then(build_named("check", &["user", "node"]));
    root = root.then(build_named("search", &["query"]));
    root = root.then(build_named("verbose", &["mode"]));
    root = root.then(build_named("tree", &["kind"]));
    root = root.then(build_named("import", &["file"]));
    root = root.then(build_named("export", &["file"]));
    root = root.then(build_named("applyedits", &["file"]));
    root = root.then(build_named("creategroup", &["name"]));
    root = root.then(build_named("deletegroup", &["name"]));
    root = root.then(build_named("createtrack", &["name"]));
    root = root.then(build_named("deletetrack", &["name"]));
    root = root.then(
        lit("listusers")
            .execute(rb(&[l("listusers")]))
            .then(arg("page", word()).execute(rb(&[l("listusers"), a("page")]))),
    );
    root = root.then(user_branch());
    root = root.then(group_branch());
    root = root.then(track_branch());
    root.then(
        arg("args", greedy())
            .suggest(Suggest)
            .execute(super::Dispatch),
    )
}

fn build_named(head: &'static str, keys: &[&'static str]) -> CommandNode {
    fn nest(head: &'static str, keys: &[&'static str], idx: usize) -> Option<CommandNode> {
        let key = *keys.get(idx)?;
        let mut pieces = vec![l(head)];
        for prev in keys.iter().take(idx + 1) {
            pieces.push(a(*prev));
        }
        let mut child = arg(key, word());
        child = match (head, key) {
            ("check", "user") => child.suggest(Names::Users),
            (h, "name") if h.contains("group") => child.suggest(Names::Groups),
            (h, "name") if h.contains("track") => child.suggest(Names::Tracks),
            ("tree", "kind") => child.suggest(Fixed(&["user", "group"])),
            ("verbose", "mode") => child.suggest(Fixed(&["on", "off", "record", "paste"])),
            _ => child,
        };
        child = child.execute(rb(&pieces));
        if let Some(inner) = nest(head, keys, idx + 1) {
            child = child.then(inner);
        } else {
            let mut more = pieces;
            more.push(a("rest"));
            child = child.then(arg("rest", greedy()).execute(rb(&more)));
        }
        Some(child)
    }
    let mut node = lit(head).execute(rb(&[l(head)]));
    if let Some(inner) = nest(head, keys, 0) {
        node = node.then(inner);
    }
    node
}

fn user_branch() -> CommandNode {
    let mut target = arg("target", word()).suggest(Names::Users);
    target = target.execute(rb(&[l("user"), a("target")]));
    target = target.then(lit("info").execute(rb(&[l("user"), a("target"), l("info")])));
    target = target.then(perm_branch("user"));
    target = target.then(parent_branch("user"));
    target = target.then(meta_branch("user"));
    target = target.then(lit("editor").execute(rb(&[l("user"), a("target"), l("editor")])));
    target = target.then(track_shift("promote"));
    target = target.then(track_shift("demote"));
    target = target.then(lit("showtracks").execute(rb(&[l("user"), a("target"), l("showtracks")])));
    target = target.then(clear_action("user"));
    target = target.then(
        lit("clone").then(
            arg("other", word())
                .suggest(Names::Users)
                .execute(rb(&[l("user"), a("target"), l("clone"), a("other")])),
        ),
    );
    lit("user").execute(rb(&[l("user")])).then(target)
}

fn group_branch() -> CommandNode {
    let mut target = arg("target", word()).suggest(Names::Groups);
    target = target.execute(rb(&[l("group"), a("target")]));
    target = target.then(lit("info").execute(rb(&[l("group"), a("target"), l("info")])));
    target = target.then(perm_branch("group"));
    target = target.then(parent_branch("group"));
    target = target.then(meta_branch("group"));
    target = target.then(lit("editor").execute(rb(&[l("group"), a("target"), l("editor")])));
    target = target.then(
        lit("listmembers")
            .execute(rb(&[l("group"), a("target"), l("listmembers")]))
            .then(arg("page", word()).execute(rb(&[l("group"), a("target"), l("listmembers"), a("page")]))),
    );
    target = target.then(
        lit("setweight").then(
            arg("weight", word()).execute(rb(&[l("group"), a("target"), l("setweight"), a("weight")])),
        ),
    );
    target = target.then(
        lit("setdisplayname").then(
            arg("display", greedy()).execute(rb(&[l("group"), a("target"), l("setdisplayname"), a("display")])),
        ),
    );
    target = target.then(lit("showtracks").execute(rb(&[l("group"), a("target"), l("showtracks")])));
    target = target.then(clear_action("group"));
    target = target.then(
        lit("rename").then(arg("new", word()).execute(rb(&[l("group"), a("target"), l("rename"), a("new")]))),
    );
    target = target.then(
        lit("clone").then(arg("new", word()).execute(rb(&[l("group"), a("target"), l("clone"), a("new")]))),
    );
    lit("group").execute(rb(&[l("group")])).then(target)
}

fn track_branch() -> CommandNode {
    let mut target = arg("target", word()).suggest(Names::Tracks);
    target = target.execute(rb(&[l("track"), a("target")]));
    for action in ["info", "editor", "clear"] {
        target = target.then(lit(action).execute(rb(&[l("track"), a("target"), l(action)])));
    }
    target = target.then(one_group("track", "append"));
    target = target.then(one_group("track", "remove"));
    target = target.then(
        lit("insert").then(
            arg("group", word()).suggest(Names::Groups).then(
                arg("position", word()).execute(rb(&[
                    l("track"),
                    a("target"),
                    l("insert"),
                    a("group"),
                    a("position"),
                ])),
            ),
        ),
    );
    target = target.then(
        lit("rename").then(arg("new", word()).execute(rb(&[l("track"), a("target"), l("rename"), a("new")]))),
    );
    target = target.then(
        lit("clone").then(arg("new", word()).execute(rb(&[l("track"), a("target"), l("clone"), a("new")]))),
    );
    lit("track").execute(rb(&[l("track")])).then(target)
}

fn one_group(kind: &'static str, action: &'static str) -> CommandNode {
    lit(action).then(
        arg("group", word())
            .suggest(Names::Groups)
            .execute(rb(&[l(kind), a("target"), l(action), a("group")])),
    )
}

fn clear_action(kind: &'static str) -> CommandNode {
    lit("clear")
        .execute(rb(&[l(kind), a("target"), l("clear")]))
        .then(arg("rest", greedy()).execute(rb(&[l(kind), a("target"), l("clear"), a("rest")])))
}

fn track_shift(action: &'static str) -> CommandNode {
    lit(action).then(
        arg("track", word())
            .suggest(Names::Tracks)
            .execute(rb(&[l("user"), a("target"), l(action), a("track")])),
    )
}

fn perm_branch(kind: &'static str) -> CommandNode {
    let mut node = lit("permission").execute(rb(&[l(kind), a("target"), l("permission")]));
    node = node.then(
        lit("info")
            .execute(rb(&[l(kind), a("target"), l("permission"), l("info")]))
            .then(arg("page", word()).execute(rb(&[
                l(kind),
                a("target"),
                l("permission"),
                l("info"),
                a("page"),
            ]))),
    );
    node = node.then(node_rest(kind, "permission", "set", true));
    node = node.then(node_rest(kind, "permission", "unset", false));
    node = node.then(node_dur_rest(kind, "permission", "settemp"));
    node = node.then(node_rest(kind, "permission", "unsettemp", false));
    node = node.then(node_rest(kind, "permission", "check", false));
    node = node.then(
        lit("clear")
            .execute(rb(&[l(kind), a("target"), l("permission"), l("clear")]))
            .then(arg("rest", greedy()).execute(rb(&[
                l(kind),
                a("target"),
                l("permission"),
                l("clear"),
                a("rest"),
            ]))),
    );
    node
}

fn parent_branch(kind: &'static str) -> CommandNode {
    let mut node = lit("parent").execute(rb(&[l(kind), a("target"), l("parent")]));
    node = node.then(lit("info").execute(rb(&[l(kind), a("target"), l("parent"), l("info")])));
    for action in ["add", "remove", "set"] {
        let mut child = lit(action).execute(rb(&[l(kind), a("target"), l("parent"), l(action)]));
        child = child.then(
            arg("group", word())
                .suggest(Names::Groups)
                .execute(rb(&[l(kind), a("target"), l("parent"), l(action), a("group")]))
                .then(arg("rest", greedy()).execute(rb(&[
                    l(kind),
                    a("target"),
                    l("parent"),
                    l(action),
                    a("group"),
                    a("rest"),
                ]))),
        );
        node = node.then(child);
    }
    node = node.then({
        let mut child = lit("addtemp").execute(rb(&[l(kind), a("target"), l("parent"), l("addtemp")]));
        child = child.then(
            arg("group", word()).suggest(Names::Groups).then(
                arg("duration", word())
                    .suggest(Fixed(DURATIONS))
                    .execute(rb(&[
                        l(kind),
                        a("target"),
                        l("parent"),
                        l("addtemp"),
                        a("group"),
                        a("duration"),
                    ]))
                    .then(arg("rest", greedy()).execute(rb(&[
                        l(kind),
                        a("target"),
                        l("parent"),
                        l("addtemp"),
                        a("group"),
                        a("duration"),
                        a("rest"),
                    ]))),
            ),
        );
        child
    });
    node = node.then(node_rest(kind, "parent", "removetemp", false));
    node = node.then(
        lit("clear")
            .execute(rb(&[l(kind), a("target"), l("parent"), l("clear")]))
            .then(arg("rest", greedy()).execute(rb(&[
                l(kind),
                a("target"),
                l("parent"),
                l("clear"),
                a("rest"),
            ]))),
    );
    if kind == "user" {
        node = node.then(
            lit("cleartrack").then(
                arg("track", word())
                    .suggest(Names::Tracks)
                    .execute(rb(&[l(kind), a("target"), l("parent"), l("cleartrack"), a("track")])),
            ),
        );
        node = node.then(
            lit("switchprimarygroup").then(
                arg("group", word())
                    .suggest(Names::Groups)
                    .execute(rb(&[
                        l(kind),
                        a("target"),
                        l("parent"),
                        l("switchprimarygroup"),
                        a("group"),
                    ])),
            ),
        );
        node = node.then(
            lit("settrack").then(
                arg("track", word()).suggest(Names::Tracks).then(
                    arg("group", word())
                        .suggest(Names::Groups)
                        .execute(rb(&[
                            l(kind),
                            a("target"),
                            l("parent"),
                            l("settrack"),
                            a("track"),
                            a("group"),
                        ])),
                ),
            ),
        );
    }
    node
}

fn meta_branch(kind: &'static str) -> CommandNode {
    let mut node = lit("meta").execute(rb(&[l(kind), a("target"), l("meta")]));
    node = node.then(lit("info").execute(rb(&[l(kind), a("target"), l("meta"), l("info")])));
    node = node.then({
        let mut child = lit("set").execute(rb(&[l(kind), a("target"), l("meta"), l("set")]));
        child = child.then(
            arg("key", word()).then(
                arg("value", word())
                    .execute(rb(&[l(kind), a("target"), l("meta"), l("set"), a("key"), a("value")]))
                    .then(arg("rest", greedy()).execute(rb(&[
                        l(kind),
                        a("target"),
                        l("meta"),
                        l("set"),
                        a("key"),
                        a("value"),
                        a("rest"),
                    ]))),
            ),
        );
        child
    });
    node = node.then(
        lit("unset")
            .execute(rb(&[l(kind), a("target"), l("meta"), l("unset")]))
            .then(arg("key", word()).execute(rb(&[l(kind), a("target"), l("meta"), l("unset"), a("key")]))),
    );
    node = node.then({
        let mut child = lit("settemp").execute(rb(&[l(kind), a("target"), l("meta"), l("settemp")]));
        child = child.then(
            arg("key", word()).then(
                arg("value", word()).then(
                    arg("duration", word())
                        .suggest(Fixed(DURATIONS))
                        .execute(rb(&[
                            l(kind),
                            a("target"),
                            l("meta"),
                            l("settemp"),
                            a("key"),
                            a("value"),
                            a("duration"),
                        ]))
                        .then(arg("rest", greedy()).execute(rb(&[
                            l(kind),
                            a("target"),
                            l("meta"),
                            l("settemp"),
                            a("key"),
                            a("value"),
                            a("duration"),
                            a("rest"),
                        ]))),
                ),
            ),
        );
        child
    });
    node = node.then(
        lit("unsettemp")
            .execute(rb(&[l(kind), a("target"), l("meta"), l("unsettemp")]))
            .then(arg("key", word()).execute(rb(&[l(kind), a("target"), l("meta"), l("unsettemp"), a("key")]))),
    );
    for action in ["addprefix", "addsuffix"] {
        node = node.then(weighted(kind, action));
    }
    for action in ["removeprefix", "removesuffix"] {
        node = node.then(
            lit(action)
                .execute(rb(&[l(kind), a("target"), l("meta"), l(action)]))
                .then(arg("priority", word()).execute(rb(&[
                    l(kind),
                    a("target"),
                    l("meta"),
                    l(action),
                    a("priority"),
                ]))),
        );
    }
    for action in ["addtempprefix", "addtempsuffix"] {
        node = node.then(weighted_temp(kind, action));
    }
    node
}

fn weighted(kind: &'static str, action: &'static str) -> CommandNode {
    lit(action)
        .execute(rb(&[l(kind), a("target"), l("meta"), l(action)]))
        .then(
            arg("priority", word()).then(
                arg("text", greedy()).execute(rb(&[
                    l(kind),
                    a("target"),
                    l("meta"),
                    l(action),
                    a("priority"),
                    a("text"),
                ])),
            ),
        )
}

fn weighted_temp(kind: &'static str, action: &'static str) -> CommandNode {
    lit(action)
        .execute(rb(&[l(kind), a("target"), l("meta"), l(action)]))
        .then(
            arg("priority", word()).then(
                arg("text", word()).then(
                    arg("duration", word())
                        .suggest(Fixed(DURATIONS))
                        .execute(rb(&[
                            l(kind),
                            a("target"),
                            l("meta"),
                            l(action),
                            a("priority"),
                            a("text"),
                            a("duration"),
                        ]))
                        .then(arg("rest", greedy()).execute(rb(&[
                            l(kind),
                            a("target"),
                            l("meta"),
                            l(action),
                            a("priority"),
                            a("text"),
                            a("duration"),
                            a("rest"),
                        ]))),
                ),
            ),
        )
}

fn node_rest(kind: &'static str, mid: &'static str, action: &'static str, bools: bool) -> CommandNode {
    let mut child = lit(action).execute(rb(&[l(kind), a("target"), l(mid), l(action)]));
    let mut node = arg("node", word()).suggest(Names::Nodes);
    node = node.execute(rb(&[l(kind), a("target"), l(mid), l(action), a("node")]));
    let rest = if bools {
        arg("rest", greedy())
            .suggest(Fixed(&["true", "false"]))
            .execute(rb(&[l(kind), a("target"), l(mid), l(action), a("node"), a("rest")]))
    } else {
        arg("rest", greedy()).execute(rb(&[l(kind), a("target"), l(mid), l(action), a("node"), a("rest")]))
    };
    node = node.then(rest);
    child = child.then(node);
    child
}

fn node_dur_rest(kind: &'static str, mid: &'static str, action: &'static str) -> CommandNode {
    let mut child = lit(action).execute(rb(&[l(kind), a("target"), l(mid), l(action)]));
    child = child.then(
        arg("node", word()).suggest(Names::Nodes).then(
            arg("duration", word())
                .suggest(Fixed(DURATIONS))
                .execute(rb(&[l(kind), a("target"), l(mid), l(action), a("node"), a("duration")]))
                .then(arg("rest", greedy()).suggest(Fixed(&["true", "false"])).execute(rb(&[
                    l(kind),
                    a("target"),
                    l(mid),
                    l(action),
                    a("node"),
                    a("duration"),
                    a("rest"),
                ]))),
        ),
    );
    child
}

const DURATIONS: &[&str] = &["30s", "15m", "1h", "1d", "7d", "30d"];

#[derive(Clone, Copy)]
enum Names {
    Users,
    Groups,
    Tracks,
    Nodes,
}

struct Fixed(&'static [&'static str]);

impl CommandSuggestionHandler for Names {
    fn suggest(
        &self,
        _sender: CommandSender,
        _server: Server,
        request: SuggestionRequest,
    ) -> CommandSuggestions {
        let options = match self {
            Names::Users => with_store(|s| s.user_names()),
            Names::Groups => with_store(|s| s.group_names()),
            Names::Tracks => with_store(|s| s.track_names()),
            Names::Nodes => with_store(|s| s.known_permissions()),
        };
        finish(request, options)
    }
}

impl CommandSuggestionHandler for Fixed {
    fn suggest(
        &self,
        _sender: CommandSender,
        _server: Server,
        request: SuggestionRequest,
    ) -> CommandSuggestions {
        finish(
            request,
            self.0.iter().map(|s| (*s).to_string()).collect(),
        )
    }
}

fn finish(request: SuggestionRequest, options: Vec<String>) -> CommandSuggestions {
    let prefix = request.remaining.to_ascii_lowercase();
    let values = options
        .into_iter()
        .filter(|v| prefix.is_empty() || v.to_ascii_lowercase().starts_with(&prefix))
        .map(|value| CommandSuggestion {
            value,
            tooltip: None,
        })
        .collect();
    CommandSuggestions {
        start: request.start,
        length: request.remaining.len() as u32,
        values,
    }
}
