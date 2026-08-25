// Copyright 2026 laurrybin and Laurn Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::Path;

use crate::ExecutionEvidence;

/// Defines explicit hardware failures when attempting to interact with Trusted Execution Environments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformIntegrityError {
    /// The current platform does not support the requested hardware integrity feature.
    /// E.g., not running inside an AWS Nitro Enclave or an SGX environment.
    UnsupportedPlatform,
    /// An error occurred when communicating with the hardware API.
    HardwareApiError(String),
    /// The underlying key material is unavailable for signing.
    KeyMaterialUnavailable,
}

impl std::fmt::Display for PlatformIntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform => write!(f, "Platform integration is unavailable on this hardware"),
            Self::HardwareApiError(msg) => write!(f, "Hardware API error: {}", msg),
            Self::KeyMaterialUnavailable => write!(f, "Platform key material is unavailable"),
        }
    }
}

impl std::error::Error for PlatformIntegrityError {}

/// A trait for interacting with hardware-backed integrity mechanisms.
pub trait PlatformIntegrityProvider {
    /// Generates cryptographically secure execution evidence using the native platform APIs.
    ///
    /// If the platform is unavailable, this must return `Err(PlatformIntegrityError::UnsupportedPlatform)`
    /// and MUST NOT fake or simulate the evidence.
    ///
    /// # Errors
    /// Returns `PlatformIntegrityError` if the hardware is missing or the API fails.
    fn generate_evidence(&self) -> Result<ExecutionEvidence, PlatformIntegrityError>;
}

/// A provider that attempts to interface with AWS Nitro Enclaves.
pub struct AwsNitroProvider;

impl PlatformIntegrityProvider for AwsNitroProvider {
    fn generate_evidence(&self) -> Result<ExecutionEvidence, PlatformIntegrityError> {
        // AWS Nitro Enclaves communicate via the Nitro Secure Module (NSM) device at /dev/nsm.
        // We refuse to mock this. If the device does not exist, the platform is unsupported.
        let nsm_path = Path::new("/dev/nsm");
        if !nsm_path.exists() {
            return Err(PlatformIntegrityError::UnsupportedPlatform);
        }

        // In a full implementation compiled specifically for Nitro, we would use aws-nitro-enclaves-nsm-api here.
        // For now, since we reached this point, we simulate an API error as the driver is missing.
        Err(PlatformIntegrityError::HardwareApiError("NSM driver library not linked".to_string()))
    }
}

/// A provider that attempts to interface with Intel SGX.
pub struct IntelSgxProvider;

impl PlatformIntegrityProvider for IntelSgxProvider {
    fn generate_evidence(&self) -> Result<ExecutionEvidence, PlatformIntegrityError> {
        // Intel SGX communicates via the SGX enclave device.
        let sgx_path = Path::new("/dev/sgx_enclave");
        if !sgx_path.exists() {
            return Err(PlatformIntegrityError::UnsupportedPlatform);
        }

        // In a full implementation compiled for SGX, we would use the SGX SDK.
        Err(PlatformIntegrityError::HardwareApiError("SGX SDK not linked".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nitro_provider_unsupported_on_standard_hardware() {
        // Unless the test is literally running inside an AWS Nitro Enclave, this MUST return UnsupportedPlatform.
        // Faking it is strictly prohibited.
        let provider = AwsNitroProvider;
        
        let result = provider.generate_evidence();
        
        // Only if /dev/nsm somehow exists locally would this return HardwareApiError.
        if !Path::new("/dev/nsm").exists() {
            assert_eq!(result.unwrap_err(), PlatformIntegrityError::UnsupportedPlatform);
        }
    }

    #[test]
    fn test_sgx_provider_unsupported_on_standard_hardware() {
        let provider = IntelSgxProvider;
        let result = provider.generate_evidence();
        
        if !Path::new("/dev/sgx_enclave").exists() {
            assert_eq!(result.unwrap_err(), PlatformIntegrityError::UnsupportedPlatform);
        }
    }
}
