use kameo::actor::Spawn;
use kameo::message::{Context, Message};
use kameo::Actor;

// Implement the actor
#[derive(Actor)]
struct Counter {
    count: i64,
}

// Define message
struct Inc {
    amount: i64,
}

// Implement message handler
impl Message<Inc> for Counter {
    type Reply = i64;

    async fn handle(&mut self, msg: Inc, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.count += msg.amount;
        self.count
    }
}

#[cfg(test)]
mod tests {
    use kameo::prelude::Spawn;

    use crate::agent::domain::test::{Counter, Inc};

    #[tokio::test]
    async fn test_run_task_cmd_multiple_messages() {
        // Spawn the actor and obtain its reference
        let actor_ref = Counter::spawn(Counter { count: 0 });

        // Send messages to the actor
        let count = actor_ref.ask(Inc { amount: 42 }).await.unwrap();
        assert_eq!(count, 42);
    }
}
