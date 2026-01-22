import { chromium } from "playwright";
import fs from "node:fs/promises";
import fsSync from "node:fs";
import { Buffer } from "node:buffer";

const args = process.argv.slice(2);
const getArg = (name) => {
  const index = args.findIndex((arg) => arg === name);
  if (index === -1) return null;
  return args[index + 1] ?? null;
};

const targetUrl = getArg("--url");
const mode = getArg("--mode") ?? "manual";
const storagePath = getArg("--storage");
const screenshotDelayArg = getArg("--screenshot-delay");
const screenshotDelayMs = screenshotDelayArg ? Number(screenshotDelayArg) : null;
const screenshotDelay = Number.isFinite(screenshotDelayMs)
  ? Math.max(0, screenshotDelayMs)
  : null;
const eventIntervalArg = getArg("--event-interval");
const eventIntervalMs = eventIntervalArg ? Number(eventIntervalArg) : null;
const eventInterval = Number.isFinite(eventIntervalMs)
  ? Math.max(0, eventIntervalMs)
  : null;

if (!targetUrl) {
  console.error("Missing target URL.");
  process.exit(1);
}

const emit = (type, payload) => {
  process.stdout.write(`${JSON.stringify({ type, payload })}\n`);
};

const safeString = (value, limit = 5000) => {
  if (!value) return null;
  const trimmed = String(value);
  return trimmed.length > limit ? `${trimmed.slice(0, limit)}...` : trimmed;
};

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const buildSelector = (el) => {
  if (!el || el.nodeType !== 1) return "";
  const element = el;
  if (element.id) {
    return `#${CSS.escape(element.id)}`;
  }
  const parts = [];
  let current = element;
  while (current && current.nodeType === 1 && parts.length < 5) {
    const tag = current.tagName.toLowerCase();
    const parent = current.parentElement;
    if (!parent) {
      parts.unshift(tag);
      break;
    }
    const siblings = Array.from(parent.children).filter(
      (child) => child.tagName.toLowerCase() === tag
    );
    const index = siblings.indexOf(current) + 1;
    parts.unshift(`${tag}:nth-of-type(${index})`);
    current = parent;
  }
  return parts.join(" > ");
};

let browser;
let context;
let page;

const saveAuthState = async () => {
  if (!storagePath || !context) return;
  try {
    await context.storageState({ path: storagePath });
    emit("auth_state", { path: storagePath });
  } catch (err) {
    emit("status", {
      level: "error",
      message: `Failed to save auth state: ${err?.message || err}`,
    });
  }
};

const shutdown = async (reason) => {
  emit("status", { level: "info", message: reason });
  await saveAuthState();
  if (browser) {
    await browser.close();
  }
  process.exit(0);
};

process.on("SIGTERM", () => void shutdown("Recorder stopping"));
process.on("SIGINT", () => void shutdown("Recorder interrupted"));

try {
  emit("status", { level: "info", message: `Launching browser (${mode})` });
  browser = await chromium.launch({ headless: false });
  context = await browser.newContext(
    storagePath && fsSync.existsSync(storagePath)
      ? { storageState: storagePath }
      : undefined
  );
  page = await context.newPage();

  let screenshotQueue = Promise.resolve();
  const emitEventWithScreenshot = async (payload) => {
    const nextPayload = { ...payload };
    if (screenshotDelay !== null) {
      if (screenshotDelay > 0) {
        await wait(screenshotDelay);
      }
      try {
        const encoded = await page.screenshot({ type: "png", encoding: "base64" });
        const base64 =
          typeof encoded === "string"
            ? encoded
            : Buffer.from(encoded).toString("base64");
        nextPayload.screenshotDataUrl = `data:image/png;base64,${base64}`;
      } catch (err) {
        emit("status", {
          level: "error",
          message: `Screenshot capture failed: ${err?.message || err}`,
        });
      }
    }
    emit("event", nextPayload);
  };

  const queueEmit = (payload) => {
    screenshotQueue = screenshotQueue
      .then(() => emitEventWithScreenshot(payload))
      .catch((err) => {
        emit("status", {
          level: "error",
          message: `Screenshot queue error: ${err?.message || err}`,
        });
        emit("event", payload);
      });
  };

  let eventQueue = [];
  let eventFlushTimer = null;

  const flushEventQueue = () => {
    if (eventFlushTimer) {
      clearTimeout(eventFlushTimer);
      eventFlushTimer = null;
    }
    if (eventQueue.length === 0) return;
    const batch = eventQueue.slice();
    eventQueue = [];
    batch.forEach((payload) => queueEmit(payload));
  };

  const scheduleEventFlush = () => {
    if (eventInterval === null || eventInterval === 0) {
      flushEventQueue();
      return;
    }
    if (eventFlushTimer) return;
    eventFlushTimer = setTimeout(() => {
      flushEventQueue();
    }, eventInterval);
  };

  await page.exposeBinding("qaRecordEvent", (_source, payload) => {
    if (eventInterval === null) {
      queueEmit(payload);
      return;
    }
    eventQueue.push(payload);
    scheduleEventFlush();
  });

  await page.addInitScript((originValue) => {
    window.__qaRecorderOrigin = originValue;
    const send = (payload) => {
      if (window.qaRecordEvent) {
        window.qaRecordEvent(payload);
      }
    };
    const buildSelector = (element) => {
      if (!element || element.nodeType !== 1) return null;
      if (element.id) return `#${CSS.escape(element.id)}`;
      const parts = [];
      let current = element;
      while (current && current.nodeType === 1 && parts.length < 5) {
        const tag = current.tagName.toLowerCase();
        const parent = current.parentElement;
        if (!parent) {
          parts.unshift(tag);
          break;
        }
        const siblings = Array.from(parent.children).filter(
          (child) => child.tagName.toLowerCase() === tag
        );
        const index = siblings.indexOf(current) + 1;
        parts.unshift(`${tag}:nth-of-type(${index})`);
        current = parent;
      }
      return parts.join(" > ");
    };
    const buildEventPayload = (eventType, target, value, meta = {}) => {
      const selector = target
        ? target.dataset?.qaSelector || buildSelector(target)
        : null;
      const text = target?.innerText ? target.innerText.slice(0, 120) : null;
      const url = window.location.href;
      const payload = {
        eventType,
        selector: selector || null,
        elementText: text,
        value: value ?? null,
        url,
        meta,
        origin: window.__qaRecorderOrigin,
      };
      send(payload);
    };

    const safeTarget = (event) => (event.target instanceof Element ? event.target : null);

    document.addEventListener(
      "click",
      (event) => {
        const target = safeTarget(event);
        if (!target) return;
        const selector = target.closest("[data-qa-selector]") || target;
        buildEventPayload("click", selector, null, {
          tag: target.tagName.toLowerCase(),
        });
      },
      true
    );

    document.addEventListener(
      "input",
      (event) => {
        const target = safeTarget(event);
        if (!target) return;
        const value =
          target instanceof HTMLInputElement ||
          target instanceof HTMLTextAreaElement ||
          target instanceof HTMLSelectElement
            ? target.value
            : null;
        buildEventPayload("input", target, value, {
          tag: target.tagName.toLowerCase(),
        });
      },
      true
    );

    document.addEventListener(
      "submit",
      (event) => {
        const target = safeTarget(event);
        if (!target) return;
        buildEventPayload("submit", target, null, {
          tag: target.tagName.toLowerCase(),
        });
      },
      true
    );
  }, mode === "ai" ? "ai" : "user");

  page.on("requestfinished", async (request) => {
    const response = await request.response();
    if (!response) return;
    const responseTiming =
      typeof response.timing === "function" ? response.timing() : null;
    const requestTiming =
      !responseTiming && typeof request.timing === "function"
        ? request.timing()
        : null;
    const timing = responseTiming ?? requestTiming;
    let timingMs = null;
    if (timing && typeof timing.responseEnd === "number") {
      timingMs = Math.max(0, timing.responseEnd - timing.requestStart);
    }
    const responseBody = await response.text().catch(() => null);
    emit("network", {
      method: request.method(),
      url: request.url(),
      status: response.status(),
      timingMs,
      requestHeaders: request.headers(),
      responseHeaders: response.headers(),
      requestBody: safeString(request.postData() ?? null),
      responseBody: safeString(responseBody),
    });
  });

  await page.goto(targetUrl, { waitUntil: "domcontentloaded" });
  emit("status", { level: "info", message: `Navigated to ${targetUrl}` });

  if (mode === "ai") {
    emit("status", { level: "info", message: "AI exploration started" });
    await page.waitForTimeout(1000);
    await page.mouse.wheel(0, 800);
    const clickable = await page.$$("button, a, [role='button']");
    for (const node of clickable.slice(0, 4)) {
      try {
        await node.click({ timeout: 4000 });
      } catch {
        // ignore
      }
      await page.waitForTimeout(800);
    }
    emit("status", { level: "info", message: "AI exploration complete" });
    await saveAuthState();
    await browser.close();
    process.exit(0);
  }
} catch (err) {
  emit("status", {
    level: "error",
    message: `Recorder failed: ${err?.message || err}`,
  });
  await saveAuthState();
  if (browser) {
    await browser.close();
  }
  process.exit(1);
}
