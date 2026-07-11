export type NonceStrategy = "timestamp" | "counter" | "highFrequency";
export type TimeInForce = "GTC" | "IOC" | "ALO";

/** Builder-code fee payload. */
export interface BuilderCodeInput {
  to: string;
  fee: number;
}

export interface OrderTypeInput {
  type: "limit" | "trigger" | "market";
  tif?: TimeInForce;
  isMarket?: boolean;
  triggerPx?: number;
}

export interface OnFillInput {
  p: number;
  actions: OrderInput[];
}

export interface OrderInput {
  type: string;
  symbol?: string;
  isBuy?: boolean;
  price?: number;
  size?: number;
  reduceOnly?: boolean;
  iso?: boolean;
  /** Optional builder-code fee payload. Omit when absent. */
  builderCode?: BuilderCodeInput;
  orderType?: OrderTypeInput;
  clientId?: string;
  orderId?: string;
  amount?: number;
  symbols?: string[];
  triggerPrice?: number;
  limitPrice?: number;
  pmin?: number;
  pmax?: number;
  lmin?: number;
  lmax?: number;
  actions?: OrderInput[];
  onFill?: OnFillInput;
  trailBps?: number;
  stepBps?: number;
}

export interface LeverageSetting {
  symbol: string;
  leverage: number;
}

export interface OraclePriceInput {
  timestamp: number;
  asset: string;
  price: number;
}

export interface PythOraclePriceInput {
  timestamp: number;
  feedIndex: number;
  price: number;
  exponent: number;
}

export interface SignedTransactionOutput {
  actions: string;
  nonce: number;
  account: string;
  signer: string;
  signature: string;
  orderId?: string;
  orderIds?: string[];
}

export interface PrepareOptions {
  account: string;
  signer?: string;
  nonce?: number;
}

export interface PreparedMessageOutput {
  messageBytes: Buffer;
  messageBase58: string;
  messageBase64: string;
  messageHex: string;
  orderId?: string;
  orderIds?: string[];
  actions: string;
  account: string;
  signer: string;
  nonce: number;
}

export class NativeKeypair {
  constructor();
  static fromBase58(s: string): NativeKeypair;
  static fromBytes(bytes: Buffer): NativeKeypair;
  readonly pubkey: string;
  toBase58(): string;
  toBytes(): Buffer;
  secretKey(): Buffer;
  cloneKeypair(): NativeKeypair;
}

export class NativeSigner {
  constructor(keypair: NativeKeypair);
  static fromBase58(s: string): NativeSigner;
  static withNonceManager(keypair: NativeKeypair, strategy: NonceStrategy): NativeSigner;
  readonly pubkey: string;
  setComputeOrderId(enabled: boolean): void;
  setComputeBatchOrderIds(enabled: boolean): void;
  computesOrderId(): boolean;
  computesBatchOrderIds(): boolean;
  sign(order: OrderInput, nonce?: number): SignedTransactionOutput;
  signAll(orders: OrderInput[], baseNonce?: number): SignedTransactionOutput[];
  signGroup(orders: OrderInput[], nonce?: number): SignedTransactionOutput;
  signFaucet(nonce?: number): SignedTransactionOutput;
  signAgentWallet(agentPubkey: string, deleteWallet: boolean, nonce?: number): SignedTransactionOutput;
  /** Approve a builder-code recipient (`abc`). */
  signApproveBuilderCode(toPubkey: string, fee: number, nonce?: number): SignedTransactionOutput;
  /** Compatibility alias for signApproveBuilderCode. */
  signApproveCommissionFee(toPubkey: string, fee: number, nonce?: number): SignedTransactionOutput;
  /** Revoke a builder-code recipient (`rbc`). */
  signRevokeBuilderCode(toPubkey: string, nonce?: number): SignedTransactionOutput;
  /** Compatibility alias for signRevokeBuilderCode. */
  signRevokeCommissionFee(toPubkey: string, nonce?: number): SignedTransactionOutput;
  signUserSettings(maxLeverage: LeverageSetting[], nonce?: number): SignedTransactionOutput;
  signOraclePrices(oracles: OraclePriceInput[], nonce?: number): SignedTransactionOutput;
  signPythOracle(oracles: PythOraclePriceInput[], nonce?: number): SignedTransactionOutput;
  signWhitelistFaucet(targetPubkey: string, whitelist: boolean, nonce?: number): SignedTransactionOutput;
  signOrder(orders: OrderInput[], nonce?: number): SignedTransactionOutput;
  signOrdersBatch(batches: OrderInput[][], baseNonce?: number): SignedTransactionOutput[];
}

export function randomHash(): string;
export function currentTimestamp(): number;
export function validatePubkey(s: string): boolean;
export function validateHash(s: string): boolean;
export function computeOrderId(wincodeBytes: Buffer): string;
export function prepareOrder(order: OrderInput, options: PrepareOptions): PreparedMessageOutput;
export function prepareAllOrders(
  orders: OrderInput[],
  options: PrepareOptions,
): PreparedMessageOutput[];
export function prepareOrderGroup(orders: OrderInput[], options: PrepareOptions): PreparedMessageOutput;
export function prepareAgentWalletAuth(
  agentPubkey: string,
  deleteWallet: boolean,
  options: PrepareOptions,
): PreparedMessageOutput;
/** Prepare a builder-code recipient approval (`abc`) for external signing. */
export function prepareApproveBuilderCode(
  toPubkey: string,
  fee: number,
  options: PrepareOptions,
): PreparedMessageOutput;
/** Compatibility alias for prepareApproveBuilderCode. */
export function prepareApproveCommissionFee(
  toPubkey: string,
  fee: number,
  options: PrepareOptions,
): PreparedMessageOutput;
/** Prepare a builder-code recipient revocation (`rbc`) for external signing. */
export function prepareRevokeBuilderCode(
  toPubkey: string,
  options: PrepareOptions,
): PreparedMessageOutput;
/** Compatibility alias for prepareRevokeBuilderCode. */
export function prepareRevokeCommissionFee(
  toPubkey: string,
  options: PrepareOptions,
): PreparedMessageOutput;
export function prepareFaucetRequest(options: PrepareOptions): PreparedMessageOutput;
export function finalizePreparedTransaction(
  prepared: PreparedMessageOutput,
  signature: string,
): SignedTransactionOutput;
