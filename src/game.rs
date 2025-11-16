use crate::events::EventHandler;

/* 
handles game state, sense, engine, position, fen, PGN, etc.
events go here. 
*/

pub struct Game {

}

impl EventHandler for Game {
    fn handle(&mut self, evt: &crate::events::Event) -> anyhow::Result<()> {
       match evt {
            _ => {todo!()}
       } 
    }
}