extern crate lapin_futures as lapin;
extern crate tokio;

use std::str::FromStr;
use std::net::ToSocketAddrs;
use tokio::prelude::*;
use tokio::prelude::Future; 
use tokio::prelude::Stream;                                                                                                                                                        
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use lapin::channel::{BasicPublishOptions, BasicConsumeOptions, BasicProperties, QueueDeclareOptions};
use lapin::client::ConnectionOptions;
use lapin::types::FieldTable;

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

                let consumer_chan = client.create_channel();
                let publisher_chan = client.create_channel();
                consumer_chan.join(publisher_chan)
            })
            .and_then(move |(consume_chan, publish_chan)| {
                println!("created consumer channel with id: {}", consume_chan.id);
                println!("created reply channel with id: {}", publish_chan.id);

                let consume_queue = consume_chan.queue_declare("primary-queue", QueueDeclareOptions::default(), FieldTable::new());
                let publish_queue = consume_chan.queue_declare("secondary-queue", QueueDeclareOptions::default(), FieldTable::new());
                
                let consume_chan = Box::new(future::result(Ok(consume_chan)));
                let publish_chan = Box::new(future::result(Ok(publish_chan)));
                
                consume_queue.join4(consume_chan, publish_queue, publish_chan)
            })
            .and_then(move |(consume_queue, consume_chan, _, publish_chan)| {
                println!("Queues are declared");
                consume_chan.basic_consume(
                    &consume_queue,
                    "",
                    BasicConsumeOptions::default(),
                    FieldTable::new())
                .and_then(move |commit_stream| {
                    println!("Stream on primary-queue");
                    
                    commit_stream.for_each(move |message| {
                        println!("Got message on primary queue. Raw: {:?}", message);
                        let consume_chan_ack = consume_chan.clone();
                        
                        // Process the message and publish the result of the process to another queue.
                        println!("Publishing processing result to secondary-queue");
                        let result = format!("processing result from {:?}", message);

                        publish_chan.basic_publish(
                            "", 
                            "secondary-queue", 
                            result.into_bytes(), 
                            BasicPublishOptions::default(), 
                            BasicProperties::default())
                        .map(|_option| ())
                        .and_then(move |()| {
                            println!("Result published. Sending ack.");
                            consume_chan_ack.basic_ack(message.delivery_tag, false)
                        })
                    })
                })
            })
    ).expect("consumer runtime exited with error");
}
