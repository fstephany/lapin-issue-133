extern crate lapin_futures as lapin;
extern crate tokio;

use std::str::FromStr;
use std::net::ToSocketAddrs;
use tokio::prelude::Future;                                                                                                                             
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use lapin::channel::{BasicPublishOptions, BasicProperties, QueueDeclareOptions};
use lapin::client::ConnectionOptions;
use lapin::types::FieldTable;
extern crate time;

fn main() {
    let mut addrs_iter = "localhost:5672".to_socket_addrs().unwrap();
    let addr = addrs_iter.next().unwrap();
    let amqp_url = "amqp://USER:PASSWORD@127.0.0.1:5672//";

    Runtime::new().unwrap().block_on(
        TcpStream::connect(&addr)
            .and_then(move |stream| {
                let conn_options = ConnectionOptions::from_str(amqp_url)
                    .expect(&format!("Could not read ConnectionOptions from: {}", amqp_url));

                println!("Connecting to RabbitMQ with username: {} password: {} vhost: {} frame_max: {} heartbeat: {}",
                    conn_options.username,
                    conn_options.password,
                    conn_options.vhost,
                    conn_options.frame_max,
                    conn_options.heartbeat);

                lapin::client::Client::connect(stream, conn_options)
            })
            .and_then(|(client, heartbeat)| {
                tokio::spawn(heartbeat.map_err(|err| {
                    println!("heartbeat error: {:?}", err);
                }));

                client.create_channel()
            })
            .and_then(|channel| {
                let id = channel.id;                
                println!("created channel with id: {}", id);

                channel
                    .queue_declare("primary-queue", QueueDeclareOptions::default(), FieldTable::new())
                    .and_then(move |_| {
                        println!("channel {} declared queue {}", id, "primary-queue");
                        let now = time::now();
                        let message = format!("Message from producer: {}", now.rfc822());
                        println!("publishing timestamp: {}", message);
                        channel.basic_publish(
                            "", 
                            "primary-queue", 
                            message.into_bytes(),
                            BasicPublishOptions::default(), 
                            BasicProperties::default())
                    })
            }))
    .expect("producer runtime exited with error");
}