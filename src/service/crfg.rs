use ed25519::Public as AuthorityId;

use inherents::{
    InherentDataProviders, RuntimeString,
};

fn register_inherent_data_provider(
    inherent_data_providers: &InherentDataProviders,
    current_authority: AuthorityId,
    last_authority_set: Vec<AuthorityId>
) -> Result<(), consensus_common::Error> {
    //consensus::register_inherent_data_provider(inherent_data_providers)?;
    if !inherent_data_providers.has_provider(&srml_crfg::INHERENT_IDENTIFIER) {
        inherent_data_providers.register_provider(srml_crfg::InherentDataProvider::new(current_authority, last_authority_set))
            .map_err(inherent_to_common_error)
    } else {
        Ok(())
    }
}

fn inherent_to_common_error(err: RuntimeString) -> consensus_common::Error {
    consensus_common::ErrorKind::InherentData(err.into()).into()
}
