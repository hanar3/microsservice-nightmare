use std::fs::File;
use std::io::Read;

use crate::SERVICE_ATTACHER;
use crate::{prelude::*, service_attacher::Attachable};
use log::debug;
use packet::{Message, PacketId, Service, LuaServices};
use rlua::{Lua, Table};
// Message is the most primitive type, it simply takes an ID and a blob of data
// Here, let's parse the message into something meaningful
pub fn parse_message(msg: Message) {
    debug!("Attempt to parse message: {:?}", msg);
    let cmd = PacketId::try_from(msg.id).unwrap();

    match cmd {
        PacketId::AttachService => {
            let service = Service::try_from(&msg.data[..])
                .map_err(|e| Error::Generic(e.to_string()))
                .unwrap();

            let attachable = Attachable::try_from(service).unwrap();

            let mut service_attacher = SERVICE_ATTACHER.write().unwrap();
            service_attacher.attach(attachable);
        },

        PacketId::LuaServices => {
            let lua_services_file = LuaServices::try_from(&msg.data[..]).unwrap();
            let filepath = std::str::from_utf8(&lua_services_file.filepath[..]).unwrap();
            let mut lua_file = match File::open(filepath) {
                Ok(file) => file,
                Err(e) => {
                    log::error!("Failed to open file {}", e.to_string());
                    return;
                }
            };

            let mut lua_script_contents = String::new();
            lua_file.read_to_string(&mut lua_script_contents).unwrap();

            drop(lua_file); // No longer needed
            let lua = Lua::new();

            // TODO: works for now, organize later!
            lua.context(|ctx| {
                ctx.load(&lua_script_contents).set_name(filepath.clone()).unwrap().exec().unwrap();
                let globals = ctx.globals();
                let table = globals.get::<_, Table>("Services").unwrap();
                for item in table.pairs::<rlua::Value, rlua::Table>(){
                    let (_, inner_table) = item.unwrap();
                    let service_name = inner_table.get::<_, String>("name").unwrap();
                    let path = inner_table.get::<_, String>("path").unwrap();
                    let port = inner_table.get::<_, i64>("port").unwrap();
                    let cmd = inner_table.get::<_, String>("command").unwrap();

                    let cmd_args = inner_table.get::<_, Table>("command_args").unwrap();
                    let mut args: Vec<String> = vec![];
                    for i in 1..=cmd_args.len().unwrap() {
                        args.push(cmd_args.get(i).unwrap());
                    }
                    debug!("service_name: {}, path: {}, port: {}, cmd: {}, args: {:?}", service_name, path, port, cmd, args);
                }
            });

        },
        PacketId::DetachService => {}
    }
}
