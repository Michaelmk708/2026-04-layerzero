import axios from 'axios';
import { $, sleep } from 'zx';

import {
    CHAIN_B_DEPLOYER,
    DEFAULT_DEPLOYER,
    EXECUTOR_ADMIN,
    FRIENDBOT_URL,
    RPC_URL,
    ZRO_DISTRIBUTOR,
} from './constants';
import { deployNativeSac, deployZroToken } from './deploy';

const CONTAINER_NAME = 'stellar-localnet';
const QUICKSTART_IMAGE = 'stellar/quickstart:testing';

// Timeout configuration (in milliseconds)
const STARTUP_TIMEOUT_MS = 300_000; // 5 minutes total timeout for localnet startup
const REQUEST_TIMEOUT_MS = 10_000; // 10 seconds per request
const RETRY_INTERVAL_MS = 2_000; // 2 seconds between retries

export async function startStellarLocalnet(): Promise<void> {
    console.log('🚀 Starting Stellar localnet...');

    // Remove any existing container first (ignore errors if not running)
    await $`docker rm -f ${CONTAINER_NAME}`.nothrow();

    // Pull image only if not available locally
    const imageExists = await $`docker image inspect ${QUICKSTART_IMAGE}`.nothrow().quiet();
    if (imageExists.exitCode !== 0) {
        console.log(`📥 Pulling ${QUICKSTART_IMAGE}...`);
        await $`docker pull ${QUICKSTART_IMAGE}`;
    }

    // Start the stellar/quickstart container directly (no Stellar CLI needed)
    await $`docker run -d --name ${CONTAINER_NAME} -p 8086:8000 ${QUICKSTART_IMAGE} --local`;

    const startTime = Date.now();

    // Wait for RPC to be healthy first (friendbot depends on it)
    console.log('⏳ Waiting for Stellar RPC to be healthy...');
    await waitForRpcHealth(startTime);
    console.log('✅ Stellar RPC is healthy');

    // Wait for friendbot to be ready and fund accounts
    console.log('⏳ Waiting for friendbot to be ready...');
    await waitForFriendbotAndFundAccounts(startTime);

    await deployNativeSac();
    await deployZroToken();
}

async function waitForRpcHealth(startTime: number): Promise<void> {
    while (Date.now() - startTime < STARTUP_TIMEOUT_MS) {
        try {
            const response = await axios.post(
                RPC_URL,
                { jsonrpc: '2.0', id: 1, method: 'getHealth' },
                {
                    timeout: REQUEST_TIMEOUT_MS,
                    headers: { 'Content-Type': 'application/json' },
                },
            );
            if (response.data?.result?.status === 'healthy') {
                return;
            }
        } catch {
            // RPC not ready yet
        }
        await sleep(RETRY_INTERVAL_MS);
    }
    throw new Error(
        `Stellar RPC failed to become healthy within ${STARTUP_TIMEOUT_MS / 1000} seconds`,
    );
}

async function waitForFriendbotAndFundAccounts(startTime: number): Promise<void> {
    while (Date.now() - startTime < STARTUP_TIMEOUT_MS) {
        try {
            // Fund DEFAULT_DEPLOYER first (doubles as friendbot readiness check)
            await fundAccount(DEFAULT_DEPLOYER.publicKey());
            console.log('✅ Friendbot ready, DEFAULT_DEPLOYER funded');

            // Fund remaining accounts in parallel
            await Promise.all([
                fundAccount(ZRO_DISTRIBUTOR.publicKey()),
                fundAccount(EXECUTOR_ADMIN.publicKey()),
                fundAccount(CHAIN_B_DEPLOYER.publicKey()),
            ]);
            console.log('✅ All accounts funded');
            return;
        } catch {
            const elapsed = Math.round((Date.now() - startTime) / 1000);
            console.log(`⏳ [${elapsed}s] Waiting for friendbot...`);
            await sleep(RETRY_INTERVAL_MS);
        }
    }
    throw new Error(`Friendbot not ready within ${STARTUP_TIMEOUT_MS / 1000} seconds`);
}

export async function fundAccount(publicKey: string): Promise<void> {
    const response = await axios.get(FRIENDBOT_URL, {
        params: {
            addr: publicKey,
        },
        timeout: REQUEST_TIMEOUT_MS,
    });

    // Check for error responses (friendbot returns 400 if already funded, which is OK)
    if (response.status >= 500) {
        throw new Error(`Friendbot returned error: ${response.status}`);
    }
}

export async function stopStellarLocalnet(): Promise<void> {
    await $`docker rm -f ${CONTAINER_NAME}`;
    console.log('✅ Stellar localnet stopped');
}
