// Smoldot
// Copyright (C) 2019-2022  Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

// This file launches a WebSocket server that proxies JSON-RPC to the
// Hydration parachain via smoldot light client.

import * as smoldot from '../dist/mjs/index-nodejs.js';
import { WebSocketServer } from 'ws';
import process from 'node:process';
import * as fs from 'node:fs';
import { Worker } from 'node:worker_threads';

// Hydration parachain and its Polkadot relay chain.
const relayChainSpec = fs.readFileSync('../../demo-chain-specs/polkadot.json', 'utf8');
const paraChainSpec = fs.readFileSync('../../demo-chain-specs/hydration.json', 'utf8');

const { port1, port2 } = new MessageChannel();
const worker = new Worker("./demo/demo-worker.mjs");
worker.on('error', (err) => { console.log("Worker error: \n" + err.message + "\n" + err.stack) });
worker.postMessage(port2, [port2]);

const client = smoldot.start({
    portToWorker: port1,
    maxLogLevel: process.stdout.isTTY ? 3 : 4,
    forbidTcp: false,
    forbidWs: false,
    forbidNonLocalWs: false,
    forbidWss: false,
    cpuRateLimit: 0.5,
    logCallback: (_level, target, message) => {
        const now = new Date();
        const hours = ("0" + now.getHours()).slice(-2);
        const minutes = ("0" + now.getMinutes()).slice(-2);
        const seconds = ("0" + now.getSeconds()).slice(-2);
        const milliseconds = ("00" + now.getMilliseconds()).slice(-3);
        console.log(
            "[%s:%s:%s.%s] [%s] %s",
            hours, minutes, seconds, milliseconds, target, message
        );
    }
});

// Start syncing the relay chain (no JSON-RPC needed) and then the parachain.
const relay = await client.addChain({
    chainSpec: relayChainSpec,
    disableJsonRpc: true,
});

const para = await client.addChain({
    chainSpec: paraChainSpec,
    potentialRelayChains: [relay],
    disableJsonRpc: true,
});

// Catch SIGINT to clean up.
process.on("SIGINT", () => {
    para.remove();
    relay.remove();
    client.terminate().then(() => process.exit(0));
});

import { createServer } from 'node:http';

// HTTP server that handles CORS preflight and upgrades to WebSocket.
const httpServer = createServer((req, res) => {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', '*');
    if (req.method === 'OPTIONS') {
        res.writeHead(204);
        res.end();
        return;
    }
    res.writeHead(200);
    res.end('Hydration RPC - use WebSocket');
});
httpServer.listen(9944);

let wsServer = new WebSocketServer({ server: httpServer });

console.log('JSON-RPC server now listening on port 9944');
console.log('Hydration RPC: ws://127.0.0.1:9944');
console.log('');

wsServer.on('connection', function (connection, request) {
    console.log('(demo) New JSON-RPC client connected: ' + request.socket.remoteAddress + '.');

    // Each WebSocket connection gets its own addChain so smoldot tracks JSON-RPC responses per connection.
    const chain = client.addChain({
        chainSpec: paraChainSpec,
        potentialRelayChains: [relay],
    }).then(chain => {
        // Forward JSON-RPC responses from smoldot to the WebSocket client.
        (async () => {
            try {
                for await (const response of chain.jsonRpcResponses) {
                    connection.send(response);
                }
            } catch (_error) { }
        })();
        return chain;
    }).catch((error) => {
        console.error("(demo) Error while adding chain: " + error);
        connection.close(1011);
    });

    connection.on('message', function (data, isBinary) {
        if (!isBinary) {
            chain
                .then(c => c.sendJsonRpc(data.toString('utf8')))
                .catch((error) => {
                    console.error("(demo) Error during JSON-RPC request: " + error);
                    process.exit(1);
                });
        } else {
            connection.close(1002);
        }
    });

    connection.on('close', function () {
        console.log("(demo) JSON-RPC client " + request.socket.remoteAddress + ' disconnected.');
        chain.then(c => c.remove()).catch(() => { });
    });
});
