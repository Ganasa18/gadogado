/**
 * QA Recorder Injectable Script
 *
 * This script is injected into external web pages to enable event recording.
 * It captures user interactions and sends them to the parent window via postMessage.
 *
 * Usage: Add this script to the target webpage or inject it dynamically.
 */

(function() {
  'use strict';

  const INPUT_DEBOUNCE_MS = 350;
  const MAX_TEXT_LENGTH = 160;

  let inputTimers = new Map();
  let scrollTimer = null;
  let resizeTimer = null;
  let lastPointer = null;
  let lastFocusedElement = null;

  // Check if already injected
  if (window.__QA_RECORDER_INJECTED__) {
    console.warn('[QA Recorder Inject] Already injected, skipping');
    return;
  }
  window.__QA_RECORDER_INJECTED__ = true;

  console.log('[QA Recorder Inject] Script loaded');

  let isReplaying = false;

  // Post event to parent window
  function postEventToParent(payload) {
    if (isReplaying) return;
    console.log('[QA Recorder Inject] Posting event:', payload.eventType);
    window.parent.postMessage({
      type: 'qa-recorder-event',
      payload: payload
    }, '*'); // In production, specify exact origin
  }

  // Build CSS selector for element
  function buildSelector(element) {
    const prioritized = ['data-testid', 'data-purpose', 'id', 'name', 'aria-label', 'role'];

    for (const attr of prioritized) {
      const value = element.getAttribute(attr);
      if (!value) continue;
      if (attr === 'id') {
        return `#${CSS.escape(value)}`;
      }
      return `${element.tagName.toLowerCase()}[${attr}="${CSS.escape(value)}"]`;
    }

    const path = [];
    let current = element;
    const rootBody = element.ownerDocument?.body;

    for (let depth = 0; current && current !== rootBody && depth < 4; depth++) {
      const tagName = current.tagName.toLowerCase();
      const index = nthOfType(current);
      path.unshift(`${tagName}:nth-of-type(${index})`);
      current = current.parentElement;
    }

    return path.length > 0 ? path.join(' > ') : undefined;
  }

  function nthOfType(element) {
    let index = 1;
    let sibling = element.previousElementSibling;
    while (sibling) {
      if (sibling.tagName === element.tagName) {
        index++;
      }
      sibling = sibling.previousElementSibling;
    }
    return index;
  }

  function getElementText(element) {
    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
      return normalizeText(
        element.getAttribute('aria-label') || element.placeholder || element.name
      );
    }

    if (element instanceof HTMLSelectElement) {
      const selected = element.selectedOptions?.[0]?.textContent;
      return normalizeText(
        selected || element.getAttribute('aria-label') || element.name
      );
    }

    return normalizeText(element.textContent);
  }

  function getElementValue(element) {
    if (element instanceof HTMLInputElement) return element.value;
    if (element instanceof HTMLTextAreaElement) return element.value;
    if (element instanceof HTMLSelectElement) return element.value;
    if (element instanceof HTMLElement && element.isContentEditable) {
      return element.innerText;
    }
    return undefined;
  }

  function maskValue(element, value) {
    if (!value || value.trim().length === 0) return undefined;
    if (element instanceof HTMLInputElement && element.type === 'password') {
      return '[masked]';
    }
    const label = [
      element.getAttribute('name'),
      element.getAttribute('id'),
      element.getAttribute('aria-label'),
    ]
      .filter(Boolean)
      .join(' ')
      .toLowerCase();
    if (label.includes('password')) {
      return '[masked]';
    }
    return value;
  }

  function normalizeText(value) {
    if (!value) return undefined;
    const trimmed = value.trim();
    if (!trimmed) return undefined;
    return trimmed.length > MAX_TEXT_LENGTH
      ? trimmed.slice(0, MAX_TEXT_LENGTH)
      : trimmed;
  }

  function stringifyMeta(meta) {
    const cleaned = {};
    Object.entries(meta).forEach(([key, value]) => {
      if (value === undefined || value === null || value === '') return;
      cleaned[key] = value;
    });
    return Object.keys(cleaned).length > 0 ? JSON.stringify(cleaned) : undefined;
  }

  function getCoordinates(event) {
    if (event instanceof MouseEvent) {
      return { x: event.clientX, y: event.clientY };
    }
    return lastPointer ?? undefined;
  }

  // Event handlers
  function handlePointerDown(event) {
    const coords = getCoordinates(event);
    if (coords) {
      lastPointer = coords;
    }
  }

  function handleClick(event) {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const isEditable =
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement ||
      (target instanceof HTMLElement && target.isContentEditable);

    postEventToParent({
      eventType: 'click',
      selector: buildSelector(target),
      elementText: getElementText(target),
      url: window.location.href,
      metaJson: stringifyMeta({
        tag: target.tagName.toLowerCase(),
        type: target instanceof HTMLInputElement ? target.type : undefined,
        isEditable,
        coordinates: getCoordinates(event),
      }),
    });
  }

  function handleInput(event) {
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (
      !(
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement
      )
    ) {
      return;
    }

    const inputType = event.inputType;
    const previousTimer = inputTimers.get(target);
    if (previousTimer) {
      clearTimeout(previousTimer);
    }

    const nextTimer = setTimeout(() => {
      inputTimers.delete(target);
      const rawValue = getElementValue(target);
      const maskedValue = maskValue(target, rawValue);

      postEventToParent({
        eventType: 'input',
        selector: buildSelector(target),
        elementText: getElementText(target),
        value: maskedValue,
        url: window.location.href,
        metaJson: stringifyMeta({
          tag: target.tagName.toLowerCase(),
          inputType,
          type: target instanceof HTMLInputElement ? target.type : undefined,
          coordinates: getCoordinates(event),
        }),
      });
    }, INPUT_DEBOUNCE_MS);

    inputTimers.set(target, nextTimer);
  }

  function handleSubmit(event) {
    const target = event.target;
    if (!(target instanceof Element)) return;

    const form = target instanceof HTMLFormElement ? target : target.closest('form');
    const element = form ?? target;

    postEventToParent({
      eventType: 'submit',
      selector: buildSelector(element),
      elementText: getElementText(element),
      url: window.location.href,
      metaJson: stringifyMeta({
        tag: element.tagName.toLowerCase(),
        action: form?.action,
        method: form?.method,
        coordinates: getCoordinates(event),
      }),
    });
  }

  function handleChange(event) {
    const target = event.target;
    if (!(target instanceof Element)) return;

    postEventToParent({
      eventType: 'change',
      selector: buildSelector(target),
      elementText: getElementText(target),
      value: getElementValue(target),
      url: window.location.href,
      metaJson: stringifyMeta({
        tag: target.tagName.toLowerCase(),
        type: target instanceof HTMLInputElement ? target.type : undefined,
        coordinates: getCoordinates(event),
      }),
    });
  }

  function handleDblClick(event) {
    const target = event.target;
    if (!(target instanceof Element)) return;

    postEventToParent({
      eventType: 'dblclick',
      selector: buildSelector(target),
      elementText: getElementText(target),
      url: window.location.href,
      metaJson: stringifyMeta({
        tag: target.tagName.toLowerCase(),
        coordinates: getCoordinates(event),
      }),
    });
  }

  function handleContextMenu(event) {
    const target = event.target;
    if (!(target instanceof Element)) return;

    postEventToParent({
      eventType: 'contextmenu',
      selector: buildSelector(target),
      elementText: getElementText(target),
      url: window.location.href,
      metaJson: stringifyMeta({
        tag: target.tagName.toLowerCase(),
        coordinates: getCoordinates(event),
      }),
    });
  }

  function handleKeyDown(event) {
    // Only capture specific control keys to avoid noise
    const capturedKeys = ['Enter', 'Escape', 'Tab', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'];
    if (!capturedKeys.includes(event.key)) return;

    const target = event.target;
    postEventToParent({
      eventType: 'keydown',
      selector: target instanceof Element ? buildSelector(target) : undefined,
      elementText: target instanceof Element ? getElementText(target) : undefined,
      value: event.key,
      url: window.location.href,
      metaJson: stringifyMeta({
        key: event.key,
        code: event.code,
        ctrlKey: event.ctrlKey,
        shiftKey: event.shiftKey,
        altKey: event.altKey,
      }),
    });
  }

  function handleScroll() {
    if (scrollTimer) clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => {
      postEventToParent({
        eventType: 'scroll',
        url: window.location.href,
        metaJson: stringifyMeta({
          scrollX: window.scrollX,
          scrollY: window.scrollY,
          timestamp: Date.now(),
        }),
      });
    }, 500);
  }

  function handleResize() {
    if (resizeTimer) clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      postEventToParent({
        eventType: 'resize',
        url: window.location.href,
        metaJson: stringifyMeta({
          innerWidth: window.innerWidth,
          innerHeight: window.innerHeight,
          outerWidth: window.outerWidth,
          outerHeight: window.outerHeight,
        }),
      });
    }, 500);
  }

  // Attach event listeners
  document.addEventListener('pointerdown', handlePointerDown, true);
  document.addEventListener('click', handleClick, true);
  document.addEventListener('dblclick', handleDblClick, true);
  document.addEventListener('contextmenu', handleContextMenu, true);
  document.addEventListener('input', handleInput, true);
  document.addEventListener('change', handleChange, true);
  document.addEventListener('submit', handleSubmit, true);
  document.addEventListener('keydown', handleKeyDown, true);
  document.addEventListener('keyup', handleKeyDown, true);
  window.addEventListener('scroll', handleScroll, { passive: true });
  window.addEventListener('resize', handleResize, { passive: true });
  document.addEventListener(
    'focusin',
    (event) => {
      if (event.target instanceof Element) {
        lastFocusedElement = event.target;
        postEventToParent({
          eventType: 'focus',
          selector: buildSelector(event.target),
          elementText: getElementText(event.target),
          url: window.location.href,
          metaJson: stringifyMeta({
            tag: event.target.tagName.toLowerCase(),
          }),
        });
      }
    },
    true
  );
  document.addEventListener(
    'focusout',
    (event) => {
      if (event.target instanceof Element) {
        postEventToParent({
          eventType: 'blur',
          selector: buildSelector(event.target),
          elementText: getElementText(event.target),
          url: window.location.href,
          metaJson: stringifyMeta({
            tag: event.target.tagName.toLowerCase(),
          }),
        });
      }
    },
    true
  );

  console.log('[QA Recorder Inject] Event listeners attached');

  function postReplayStatus(type, payload) {
    window.parent.postMessage(
      {
        type,
        ...payload,
      },
      '*'
    );
  }

  function dispatchMouseEvent(target, type) {
    const evt = new MouseEvent(type, {
      bubbles: true,
      cancelable: true,
      view: window,
    });
    target.dispatchEvent(evt);
  }

  function applyReplayValue(target, value) {
    if (value === undefined || value === null) return;
    if (
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement
    ) {
      target.value = value;
      return;
    }
    if (target instanceof HTMLElement && target.isContentEditable) {
      target.innerText = value;
    }
  }

  function replayEvent(payload) {
    const eventType = payload?.eventType;
    const selector = payload?.selector;
    const value = payload?.value;
    const url = payload?.url;

    if (eventType === 'navigation' && url) {
      window.location.assign(url);
      return;
    }

    if (!selector) {
      throw new Error('Replay requires a selector.');
    }

    let target = null;
    try {
      target = document.querySelector(selector);
    } catch (err) {
      throw new Error('Replay selector is invalid.');
    }

    if (!target) {
      throw new Error('Replay target element not found.');
    }

    isReplaying = true;
    try {
      if (eventType === 'click') {
        if (typeof target.focus === 'function') {
          target.focus({ preventScroll: true });
        }
        dispatchMouseEvent(target, 'pointerdown');
        dispatchMouseEvent(target, 'mousedown');
        dispatchMouseEvent(target, 'mouseup');
        dispatchMouseEvent(target, 'click');
        return;
      }

      if (eventType === 'input' || eventType === 'change') {
        if (typeof target.focus === 'function') {
          target.focus({ preventScroll: true });
        }
        applyReplayValue(target, value);
        target.dispatchEvent(new Event('input', { bubbles: true }));
        target.dispatchEvent(new Event('change', { bubbles: true }));
        return;
      }

      if (eventType === 'submit') {
        const form =
          target instanceof HTMLFormElement ? target : target.closest('form');
        if (!form) {
          throw new Error('Replay submit target not found.');
        }
        const submitEvent = new Event('submit', {
          bubbles: true,
          cancelable: true,
        });
        const didDispatch = form.dispatchEvent(submitEvent);
        if (didDispatch && typeof form.submit === 'function') {
          form.submit();
        }
        return;
      }

      if (eventType === 'focus') {
        if (typeof target.focus === 'function') {
          target.focus({ preventScroll: true });
        }
        return;
      }

      if (eventType === 'blur') {
        if (typeof target.blur === 'function') {
          target.blur();
        }
        return;
      }
    } finally {
      isReplaying = false;
    }

    throw new Error(`Replay not supported for ${eventType}.`);
  }

  function handleParentCommand(event) {
    if (event.source !== window.parent) return;
    if (!event.data || event.data.type !== 'qa-recorder-command') return;

    const action = event.data.action;
    if (action === 'back') {
      window.history.back();
      return;
    }
    if (action === 'refocus') {
      if (lastFocusedElement && document.contains(lastFocusedElement)) {
        if (typeof lastFocusedElement.focus === 'function') {
          lastFocusedElement.focus({ preventScroll: true });
        }
      }
      return;
    }
    if (action === 'capture') {
      const requestId = event.data.requestId;
      captureDocumentAsDataUrl()
        .then((dataUrl) => {
          window.parent.postMessage(
            { type: 'qa-recorder-capture', requestId, dataUrl },
            '*'
          );
        })
        .catch((err) => {
          window.parent.postMessage(
            {
              type: 'qa-recorder-capture-error',
              requestId,
              error: err?.message || 'Failed to capture preview.',
            },
            '*'
          );
        });
      return;
    }
    if (action === 'replay-event') {
      const payload = event.data.payload || {};
      try {
        replayEvent(payload);
        postReplayStatus('qa-recorder-replay', {
          eventId: payload.eventId,
          eventType: payload.eventType,
        });
      } catch (err) {
        postReplayStatus('qa-recorder-replay-error', {
          eventId: payload.eventId,
          eventType: payload.eventType,
          error: err?.message || 'Replay failed.',
        });
      }
    }
  }

  async function captureDocumentAsDataUrl() {
    try {
      return await renderDocumentToDataUrl(document.documentElement);
    } catch (err) {
      if (isTaintedCanvasError(err)) {
        try {
          const sanitized = sanitizeDocumentElement(document.documentElement);
          return await renderDocumentToDataUrl(sanitized);
        } catch (sanitizeErr) {
          console.warn('[QA Recorder Inject] Sanitized capture also failed, using fallback:', sanitizeErr);
          return generateFallbackDataUrl();
        }
      }
      console.warn('[QA Recorder Inject] Capture failed with unexpected error, using fallback:', err);
      return generateFallbackDataUrl();
    }
  }

  function generateFallbackDataUrl() {
    const safeWidth = Math.max(100, window.innerWidth || 800);
    const safeHeight = Math.max(100, window.innerHeight || 600);
    const canvas = document.createElement('canvas');
    canvas.width = safeWidth;
    canvas.height = safeHeight;
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      // Return a minimal 1x1 transparent PNG
      return 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=';
    }

    // Draw a styled placeholder
    ctx.fillStyle = '#1e1e2e';
    ctx.fillRect(0, 0, safeWidth, safeHeight);

    // Add diagonal stripes pattern
    ctx.strokeStyle = '#2a2a3e';
    ctx.lineWidth = 2;
    for (let i = -safeHeight; i < safeWidth; i += 20) {
      ctx.beginPath();
      ctx.moveTo(i, 0);
      ctx.lineTo(i + safeHeight, safeHeight);
      ctx.stroke();
    }

    // Add centered text
    ctx.fillStyle = '#6c7086';
    ctx.font = '14px -apple-system, BlinkMacSystemFont, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('Preview capture unavailable', safeWidth / 2, safeHeight / 2 - 10);

    // Add smaller subtext
    ctx.font = '11px -apple-system, BlinkMacSystemFont, sans-serif';
    ctx.fillStyle = '#585b70';
    ctx.fillText('Cross-origin content blocked', safeWidth / 2, safeHeight / 2 + 10);

    return canvas.toDataURL('image/png');
  }

  function isTaintedCanvasError(err) {
    const message = err?.message || '';
    return (
      message.includes('Tainted canvases') ||
      message.includes('SecurityError')
    );
  }

  function sanitizeDocumentElement(root) {
    const clone = root.cloneNode(true);
    
    // Strip elements that commonly cause cross-origin issues
    const stripSelectors = [
      'img',
      'picture',
      'source',
      'video',
      'audio',
      'canvas',
      'iframe',
      'svg',
      'object',
      'embed',
      'link[rel="stylesheet"]',
      'link[rel="icon"]',
      'link[rel="preload"]',
      'script',
    ];
    clone.querySelectorAll(stripSelectors.join(',')).forEach((el) => el.remove());

    // Strip font-face and all url() references from style elements
    clone.querySelectorAll('style').forEach((style) => {
      if (!style.textContent) return;
      let text = style.textContent;
      text = text.replace(/@font-face\s*\{[\s\S]*?\}/gi, '');
      text = text.replace(/@import\s+[^;]+;/gi, '');
      text = text.replace(/url\s*\([^)]*\)/gi, 'none');
      style.textContent = text;
    });

    // Strip url() references from inline styles
    clone.querySelectorAll('[style]').forEach((el) => {
      const inline = el.getAttribute('style');
      if (!inline) return;
      const cleaned = inline.replace(/url\s*\([^)]*\)/gi, 'none');
      el.setAttribute('style', cleaned);
    });

    // Strip src/srcset attributes that might point to external resources
    clone.querySelectorAll('[src], [srcset]').forEach((el) => {
      el.removeAttribute('src');
      el.removeAttribute('srcset');
    });

    // Strip background attributes (old HTML)
    clone.querySelectorAll('[background]').forEach((el) => {
      el.removeAttribute('background');
    });

    return clone;
  }

  async function renderDocumentToDataUrl(root) {
    const safeWidth = Math.max(1, Math.floor(window.innerWidth));
    const safeHeight = Math.max(1, Math.floor(window.innerHeight));
    const serialized = new XMLSerializer().serializeToString(root);
    const wrapped = `<div xmlns="http://www.w3.org/1999/xhtml">${serialized}</div>`;
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${safeWidth}" height="${safeHeight}"><foreignObject width="100%" height="100%">${wrapped}</foreignObject></svg>`;
    const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(blob);

    try {
      const img = await new Promise((resolve, reject) => {
        const image = new Image();
        image.onload = () => resolve(image);
        image.onerror = () => reject(new Error('Failed to render preview snapshot.'));
        image.src = url;
      });

      const canvas = document.createElement('canvas');
      canvas.width = safeWidth;
      canvas.height = safeHeight;
      const ctx = canvas.getContext('2d');
      if (!ctx) {
        throw new Error('Canvas is not available for screenshot.');
      }
      ctx.drawImage(img, 0, 0, safeWidth, safeHeight);
      return canvas.toDataURL('image/png');
    } finally {
      URL.revokeObjectURL(url);
    }
  }

  window.addEventListener('message', handleParentCommand);

  // Notify parent that recorder is ready
  window.parent.postMessage({
    type: 'qa-recorder-ready'
  }, '*');
})();
