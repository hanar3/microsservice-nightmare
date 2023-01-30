use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

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
        // Loads the services from a lua file
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
                let services = globals.get::<_, Table>("Services").unwrap();
                
                let mut service_attacher = SERVICE_ATTACHER.write().unwrap();
                // Parsing Lua services. TODO: Better error handling / reporting
                // Avoid just unwraping here...actually handle nils from lua
                for item in services.pairs::<rlua::Value, rlua::Table>(){
                    let (_, service) = item.unwrap();
                    let service_name = service.get::<_, String>("name").unwrap();
                    let path = service.get::<_, String>("path").unwrap();
                    let port = service.get::<_, u16>("port").unwrap();
                    let service_type = service.get::<_, u8>("service_type").unwrap();
                    let cmd = service.get::<_, String>("command").unwrap();

                    // Extract command_args (lua table) into the args vector...is there a better
                    // way to do this?                   
                    let cmd_args = service.get::<_, Table>("command_args").unwrap();
                    let mut args: Vec<std::string::String> = vec![];
                    for i in 1..=cmd_args.len().unwrap() {
                        args.push(cmd_args.get::<_, String>(i).unwrap());
                    }

                    let attachable = Attachable::new(service_name.clone(), cmd.clone(), args.clone(), PathBuf::from(path.clone()), service_type, port); 
                    service_attacher.attach(attachable);
                    debug!("service_name: {}, path: {}, port: {}, cmd: {}, args: {:?}, service_type: {}", service_name, path, port, cmd, args, service_type);
                }
            });

        },
        PacketId::DetachService => {}
    }
}
