## Lapin issue 133

The goal of this project is to illustrate an issue at startup when trying 
to setup RabbitMQ processing.

    $ cargo run --bin producer
    $ cargo run --bin consumer

Sometimes, the consumer does not consume the messages that are already present 
in the queue. 

## Reproduce

- run the producer a few times (for example 3 times): `cargo run --bin producer` 
- run the consumer (`cargo run --bin consumer`). It is supposed to print *Got 
  message on primary queue.* with the details of the message.

In many situation, the consumer get stuck at *Stream on primary-queue* and does 
not process the message (even though the RabbitMQ admin panel tells us they are
there).

An interesting thing is that the RabbitMQ admin tells us that one message is 
*unacked* in the `primary-queue`. So the consumer has fetched it but it never
goes farther and the *Got message...* is never printed. 

You need to run the consumer a few times to get it to work. It looks like some
threading (starving? deadlock?) issue.

If the consumer manage to start and to process the first message, there seems 
that it will process new messages as expected and won't be stuck again.

## Implementation

The producer simply publish a message in the `primary-queue`.

The consumer creates two channels, one to interact with the `primary-queue` and
one to interact with the `secondary-queue`. It `basic_consume()` the 
`primary-queue`, simulate some work on the message, send the result to the 
`secondary-queue` and then `basic_ack()` the initial message from the 
`primary_queue`.