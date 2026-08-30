#[path = "../structured_contract_artifacts.rs"]
mod runtime_structured_contract_artifacts;
#[path = "../support/mod.rs"]
mod support;

mod system {
    use crate::runtime_structured_contract_artifacts as structured_contract_artifacts;

    mod bootstrap_package_admission {
        include!("../bootstrap_package_admission.rs");
    }
    mod pypi_environment_admission {
        include!("../pypi_environment_admission.rs");
    }
    mod release_actions_lifecycle {
        include!("../release_actions_lifecycle.rs");
    }
    mod release_changelog_script {
        include!("../release_changelog_script.rs");
    }
    mod release_lifecycle_contract {
        include!("../release_lifecycle_contract.rs");
    }
    #[cfg(unix)]
    mod release_publication_recovery {
        include!("../release_publication_recovery.rs");
    }
    mod release_publisher_changelog {
        include!("../release_publisher_changelog.rs");
    }
    mod release_tag_parity {
        include!("../release_tag_parity.rs");
    }
    mod release_workflow_parity {
        include!("../release_workflow_parity.rs");
    }
    mod runtime_activation_branch_recovery {
        include!("../runtime_activation_branch_recovery.rs");
    }
    mod runtime_candidate_assembly_contract {
        include!("../runtime_candidate_assembly_contract.rs");
    }
    mod runtime_platform_detection {
        include!("../runtime_platform_detection.rs");
    }
    mod runtime_publication_activation {
        include!("../runtime_publication_activation.rs");
    }
    mod runtime_workflow_recovery {
        include!("../runtime_workflow_recovery.rs");
    }
    mod runtime_wrapper_fallback_order {
        include!("../runtime_wrapper_fallback_order.rs");
    }
    mod windows_mcp_install_workflow_contract {
        include!("../windows_mcp_install_workflow_contract.rs");
    }
}
