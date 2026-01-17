/**
 * Playwright Screenshot Capture Script
 *
 * Captures full-page screenshots of a URL by scrolling and taking tiles.
 * Outputs PNG tiles suitable for OCR processing.
 *
 * Usage: node playwright-capture.js <url> <output_dir>
 *
 * Output:
 * - tiles/tile-001.png, tile-002.png, etc.
 * - manifest.json with capture metadata
 */

import { chromium } from 'playwright';
import path from 'path';
import fs from 'fs';

// Configuration
const CONFIG = {
    viewportWidth: 1280,
    viewportHeight: 2000,
    deviceScaleFactor: 2,
    tileOverlap: 120,
    timeout: 60000,
    waitForNetworkIdle: true,
};

async function captureUrl(url, outputDir) {
    const tilesDir = path.join(outputDir, 'tiles');

    // Ensure output directories exist
    if (!fs.existsSync(outputDir)) {
        fs.mkdirSync(outputDir, { recursive: true });
    }
    if (!fs.existsSync(tilesDir)) {
        fs.mkdirSync(tilesDir, { recursive: true });
    }

    const browser = await chromium.launch({
        headless: true,
    });

    const context = await browser.newContext({
        viewport: {
            width: CONFIG.viewportWidth,
            height: CONFIG.viewportHeight,
        },
        deviceScaleFactor: CONFIG.deviceScaleFactor,
        userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });

    const page = await context.newPage();

    console.log(JSON.stringify({ status: 'navigating', url }));

    try {
        await page.goto(url, {
            waitUntil: CONFIG.waitForNetworkIdle ? 'networkidle' : 'load',
            timeout: CONFIG.timeout,
        });
    } catch (err) {
        // If networkidle times out, try with just load
        if (err.message.includes('timeout')) {
            console.log(JSON.stringify({ status: 'retrying', reason: 'networkidle timeout' }));
            await page.goto(url, {
                waitUntil: 'load',
                timeout: CONFIG.timeout,
            });
        } else {
            throw err;
        }
    }

    // Wait a bit for any lazy-loaded content
    await page.waitForTimeout(1000);

    // Get full page dimensions
    const dimensions = await page.evaluate(() => {
        return {
            width: Math.max(
                document.body.scrollWidth,
                document.documentElement.scrollWidth,
                document.body.offsetWidth,
                document.documentElement.offsetWidth,
                document.body.clientWidth,
                document.documentElement.clientWidth
            ),
            height: Math.max(
                document.body.scrollHeight,
                document.documentElement.scrollHeight,
                document.body.offsetHeight,
                document.documentElement.offsetHeight,
                document.body.clientHeight,
                document.documentElement.clientHeight
            ),
        };
    });

    console.log(JSON.stringify({ status: 'dimensions', ...dimensions }));

    const tiles = [];
    const effectiveHeight = CONFIG.viewportHeight - CONFIG.tileOverlap;
    const numTiles = Math.ceil(dimensions.height / effectiveHeight);

    console.log(JSON.stringify({ status: 'capturing', numTiles }));

    for (let i = 0; i < numTiles; i++) {
        const yOffset = i * effectiveHeight;
        const tileNum = String(i + 1).padStart(3, '0');
        const tilePath = path.join(tilesDir, `tile-${tileNum}.png`);

        // Scroll to position
        await page.evaluate((y) => window.scrollTo(0, y), yOffset);
        await page.waitForTimeout(200); // Wait for scroll and lazy content

        // Take screenshot
        await page.screenshot({
            path: tilePath,
            clip: {
                x: 0,
                y: 0,
                width: CONFIG.viewportWidth,
                height: CONFIG.viewportHeight,
            },
        });

        tiles.push({
            index: i,
            path: path.relative(outputDir, tilePath),
            yOffset,
            width: CONFIG.viewportWidth * CONFIG.deviceScaleFactor,
            height: CONFIG.viewportHeight * CONFIG.deviceScaleFactor,
        });

        console.log(JSON.stringify({ status: 'tile_captured', index: i, total: numTiles }));
    }

    // Get page title and metadata
    const pageTitle = await page.title();
    const pageUrl = page.url();

    await browser.close();

    // Create manifest
    const manifest = {
        engine: 'playwright',
        version: '1.0.0',
        timestamp: new Date().toISOString(),
        url: pageUrl,
        originalUrl: url,
        title: pageTitle,
        viewport: {
            width: CONFIG.viewportWidth,
            height: CONFIG.viewportHeight,
            deviceScaleFactor: CONFIG.deviceScaleFactor,
        },
        pageSize: dimensions,
        tileOverlap: CONFIG.tileOverlap,
        tiles,
    };

    const manifestPath = path.join(outputDir, 'manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

    console.log(JSON.stringify({
        status: 'complete',
        tilesCount: tiles.length,
        manifestPath: path.relative(outputDir, manifestPath),
    }));

    return manifest;
}

// Main execution
async function main() {
    const args = process.argv.slice(2);

    if (args.length < 2) {
        console.error(JSON.stringify({
            error: 'Missing arguments',
            usage: 'node playwright-capture.js <url> <output_dir>',
        }));
        process.exit(1);
    }

    const [url, outputDir] = args;

    try {
        const manifest = await captureUrl(url, outputDir);
        // Final output for parsing
        console.log('---RESULT---');
        console.log(JSON.stringify(manifest));
    } catch (err) {
        console.error(JSON.stringify({
            error: err.message,
            stack: err.stack,
        }));
        process.exit(1);
    }
}

main();
