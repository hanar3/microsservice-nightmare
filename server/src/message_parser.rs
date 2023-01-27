use crate::SERVICE_ATTACHER;
use crate::{prelude::*, service_attacher::Attachable};
use packet::{Message, PacketId, Service};

// Message is the most primitive type, it simply takes an ID and a blob of data
// Here, let's parse the message into something meaningful
pub async fn parse_message(msg: Message) {
    let cmd = PacketId::try_from(msg.id).unwrap();

    match cmd {
        PacketId::AttachService => {
            let service = Service::try_from(&msg.data[..])
                .map_err(|e| Error::Generic(e.to_string()))
                .unwrap();

            let attachable = Attachable::try_from(service).unwrap();

            let mut service_attacher = SERVICE_ATTACHER.write().await;
            service_attacher.attach(attachable);
        }
        PacketId::DetachService => {}
    }
}
