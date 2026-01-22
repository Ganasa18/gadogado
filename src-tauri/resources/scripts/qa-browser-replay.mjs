import { chromium } from "playwright";
import fs from "node:fs/promises";

const payloadPath = process.argv[2];
if (!payloadPath) {
  console.error("Missing replay payload path.");
  process.exit(1);
}

const rawPayload = await fs.readFile(payloadPath, "utf-8");
const payload = JSON.parse(rawPayload);
const events = Array.isArray(payload?.events) ? payload.events : [];
const targetUrl = typeof payload?.targetUrl === "string" ? payload.targetUrl : "";

if (events.length === 0) {
  console.log("No events to replay.");
  process.exit(0);
}

const sortedEvents = [...events].sort((left, right) => (left.seq ?? 0) - (right.seq ?? 0));
const firstUrl =
  targetUrl ||
  sortedEvents.find((event) => typeof event.url === "string" && event.url)?.url ||
  "";

if (!firstUrl) {
  console.error("Missing target URL for replay.");
  process.exit(1);
}

const delayBetween = (current, next) => {
  const currentTs = typeof current?.ts === "number" ? current.ts : 0;
  const nextTs = typeof next?.ts === "number" ? next.ts : 0;
  const delta = nextTs - currentTs;
  return Number.isFinite(delta) && delta > 0 ? Math.max(200, delta) : 400;
};

const SELECTOR_TIMEOUT_MS = 10000;

const withStepGuard = async (label, action) => {
  try {
    await action();
    return true;
  } catch (err) {
    console.error(`${label} failed: ${err?.message || err}`);
    return false;
  }
};

let browser;
try {
  console.log("Launching Chromium...");
  browser = await chromium.launch({ headless: false });
  const context = await browser.newContext();
  const page = await context.newPage();

  await page.goto(firstUrl, { waitUntil: "domcontentloaded" });
  console.log(`Navigated to ${firstUrl}`);

  for (let index = 0; index < sortedEvents.length; index += 1) {
    const event = sortedEvents[index];
    const eventType = String(event.eventType ?? event.event_type ?? "").toLowerCase();
    const selector = typeof event.selector === "string" ? event.selector : "";
    const value = event.value ?? "";
    const eventUrl = typeof event.url === "string" ? event.url : "";

    if (eventType === "navigation") {
      const nextUrl = eventUrl || firstUrl;
      console.log(`Navigate -> ${nextUrl}`);
      await withStepGuard("Navigation", async () => {
        await page.goto(nextUrl, { waitUntil: "domcontentloaded" });
      });
    } else if (eventType === "click") {
      if (!selector) {
        console.error("Missing selector for click.");
      } else {
        console.log(`Click -> ${selector}`);
        await withStepGuard(`Click ${selector}`, async () => {
          const locator = page.locator(selector).first();
          await locator.waitFor({ state: "visible", timeout: SELECTOR_TIMEOUT_MS });
          await locator.click({ timeout: SELECTOR_TIMEOUT_MS });
        });
      }
    } else if (eventType === "input" || eventType === "change") {
      if (!selector) {
        console.error("Missing selector for input.");
      } else {
        console.log(`Fill -> ${selector}`);
        await withStepGuard(`Fill ${selector}`, async () => {
          const locator = page.locator(selector).first();
          await locator.waitFor({ state: "visible", timeout: SELECTOR_TIMEOUT_MS });
          await locator.fill(String(value ?? ""), { timeout: SELECTOR_TIMEOUT_MS });
        });
      }
    } else if (eventType === "focus") {
      if (!selector) {
        console.error("Missing selector for focus.");
      } else {
        console.log(`Focus -> ${selector}`);
        await withStepGuard(`Focus ${selector}`, async () => {
          const locator = page.locator(selector).first();
          await locator.waitFor({ state: "visible", timeout: SELECTOR_TIMEOUT_MS });
          await locator.focus({ timeout: SELECTOR_TIMEOUT_MS });
        });
      }
    } else if (eventType === "blur") {
      if (!selector) {
        console.error("Missing selector for blur.");
      } else {
        console.log(`Blur -> ${selector}`);
        await withStepGuard(`Blur ${selector}`, async () => {
          await page.evaluate((sel) => {
            const el = document.querySelector(sel);
            if (el && typeof el.blur === "function") {
              el.blur();
            }
          }, selector);
        });
      }
    } else if (eventType === "submit") {
      if (!selector) {
        console.error("Missing selector for submit.");
      } else {
        console.log(`Submit -> ${selector}`);
        await withStepGuard(`Submit ${selector}`, async () => {
          await page.evaluate((sel) => {
            const el = document.querySelector(sel);
            if (!el) return;
            if (el instanceof HTMLFormElement) {
              if (typeof el.requestSubmit === "function") {
                el.requestSubmit();
              } else {
                el.submit();
              }
              return;
            }
            const form = el.closest("form");
            if (form instanceof HTMLFormElement) {
              if (typeof form.requestSubmit === "function") {
                form.requestSubmit();
              } else {
                form.submit();
              }
            }
          }, selector);
        });
      }
    } else {
      console.log(`Skipping unsupported event: ${eventType}`);
    }

    if (index < sortedEvents.length - 1) {
      const delayMs = delayBetween(event, sortedEvents[index + 1]);
      await page.waitForTimeout(delayMs);
    }
  }

  console.log("Replay complete.");
} catch (err) {
  console.error(`Replay failed: ${err?.message || err}`);
  process.exitCode = 1;
} finally {
  if (browser) {
    await browser.close();
  }
}
