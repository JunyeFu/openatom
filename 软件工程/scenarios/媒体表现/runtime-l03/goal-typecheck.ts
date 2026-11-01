import type { FactoryGenerationGoal } from '../../../packages/commands/generated/typescript/FactoryGenerationGoal';
type Additional = FactoryGenerationGoal['additional_acceptance_scopes'];
const missing: Additional = undefined;
const empty: Additional = [];
// @ts-expect-error Rust Vec rejects explicit null; generated optional list must agree.
const nullList: Additional = null;
// @ts-expect-error A list element must be a complete canonical AcceptanceScope.
const incomplete: Additional = [{}];
