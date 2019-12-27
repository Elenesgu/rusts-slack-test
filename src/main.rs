use slack;
use slack::{Event, RtmClient};
use slack::api::{Message, MessageStandard};

struct MyHandler;

impl MyHandler {
    fn send(&mut self, cli: &RtmClient, event: Event) {
        let to_send_channel;
        let to_send_msg;

        match event {
            Event::Message(message) => {
                match *message {
                    Message::Standard(MessageStandard {
                                          ref channel,
                                          ref text,
                                          ..
                                      }) => {
                        match channel {
                            Some(s) => to_send_channel = s.clone(),
                            None =>  panic!("channel invalid."),
                        }
                        to_send_msg = "Hello";
                        println!("sended: {:?}", text);
                    }
                    _ => panic!("Message decoded into incorrect variant."),
                }
            }
            _ => panic!("Event decoded into incorrect variant."),
        }

        cli.sender().send_message(&*to_send_channel, to_send_msg).expect("foo");
    }
}

#[allow(unused_variables)]
impl slack::EventHandler for MyHandler {

    fn on_event(&mut self, cli: &RtmClient, event: Event) {
        match event {
            Event::Message(_) => MyHandler::send(self, cli, event),
            _ => {},
        }        
    }

    fn on_close(&mut self, cli: &RtmClient) {
        println!("on_close");
    }

    fn on_connect(&mut self, cli: &RtmClient) {
        println!("on_connect");
        // find the general channel id from the `StartResponse`
        let general_channel_id = cli.start_response()
            .channels
            .as_ref()
            .and_then(|channels| {
                          channels
                              .iter()
                              .find(|chan| match chan.name {
                                        None => false,
                                        Some(ref name) => name == "testrustslackapp",
                                    })
                      })
            .and_then(|chan| chan.id.as_ref())
            .expect("general channel not found");
        println!("channel: {:?}", general_channel_id);
        let foo = cli.sender().send_message(&general_channel_id, "Hello world! (rtm)");
        println!("send: {:?}", foo);
        // Send a message over the real time api websocket
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let api_key = match args.len() {
        0 | 1 => panic!("No api-key in args! Usage: cargo run --example slack_example -- <api-key>"),
        x => args[x - 1].clone(),
    };
    let mut handler = MyHandler;
    let r = RtmClient::login_and_run(&api_key, &mut handler);
    match r {
        Ok(_) => {}
        Err(err) => panic!("Error: {}", err),
    }
}
