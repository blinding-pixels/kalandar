import type {
  CapturePreviewContractVersion,
  CapturePreviewImplementation,
  CapturePreviewImplementationDescriptor,
} from './types.ts';

export const CAPTURE_PREVIEW_CONTRACT_VERSION: CapturePreviewContractVersion = 'capture-preview-v1';

export const CAPTURE_PREVIEW_IMPLEMENTATIONS = {
  typescript: {
    parser: 'capture',
    implementation: 'typescript',
    contract_version: CAPTURE_PREVIEW_CONTRACT_VERSION,
    boundary: 'in-process',
    host: 'javascript',
    status: 'ready',
  },
  'rust-cli': {
    parser: 'capture',
    implementation: 'rust-cli',
    contract_version: CAPTURE_PREVIEW_CONTRACT_VERSION,
    boundary: 'json-stdio-cli',
    host: 'native',
    status: 'ready',
  },
  coreai: {
    parser: 'capture',
    implementation: 'coreai',
    contract_version: CAPTURE_PREVIEW_CONTRACT_VERSION,
    boundary: 'native-host-bridge',
    host: 'apple-native',
    status: 'host-required',
  },
} satisfies Record<CapturePreviewImplementation, CapturePreviewImplementationDescriptor>;

export const capturePreviewImplementationDescriptor = (
  implementation: CapturePreviewImplementation,
): CapturePreviewImplementationDescriptor => CAPTURE_PREVIEW_IMPLEMENTATIONS[implementation];
