use crate::resource::persistence::error::PersistenceError;
use opendut_types::peer::state::PeerMemberState;
use opendut_types::peer::PeerId;
use crate::resource::api::resources::Resources;

impl Resources<'_> {
    pub async fn get_peer_member_state(&mut self, peer_id: PeerId) -> Result<Option<PeerMemberState>, PersistenceError> {
        let peer_member_states = self.list_peer_member_states().await?;
        Ok(peer_member_states.get(&peer_id).cloned())
    }
}
