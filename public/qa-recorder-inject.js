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
  let lastPointer = null;
  let lastFocusedElement = null;

  // Check if already injected
  if (window.__QA_RECORDER_INJECTED__) {
    console.warn('[QA Recorder Inject] Already injected, skipping');
    return;
  }
  window.__QA_RECORDER_INJECTED__ = true;

  console.log('[QA Recorder Inject] Script loaded');

  // Post event to parent window
  function postEventToParent(payload) {
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

  // Attach event listeners
  document.addEventListener('pointerdown', handlePointerDown, true);
  document.addEventListener('click', handleClick, true);
  document.addEventListener('input', handleInput, true);
  document.addEventListener('submit', handleSubmit, true);
  document.addEventListener(
    'focusin',
    (event) => {
      if (event.target instanceof Element) {
        lastFocusedElement = event.target;
      }
    },
    true
  );

  console.log('[QA Recorder Inject] Event listeners attached');

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
    }
  }

  async function captureDocumentAsDataUrl() {
    try {
      return await renderDocumentToDataUrl(document.documentElement);
    } catch (err) {
      if (isTaintedCanvasError(err)) {
        const sanitized = sanitizeDocumentElement(document.documentElement);
        return renderDocumentToDataUrl(sanitized);
      }
      throw err;
    }
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
    const stripSelectors = [
      'img',
      'picture',
      'source',
      'video',
      'audio',
      'canvas',
      'iframe',
      'svg',
      'link[rel="stylesheet"]',
    ];
    clone.querySelectorAll(stripSelectors.join(',')).forEach((el) => el.remove());

    clone.querySelectorAll('style').forEach((style) => {
      if (!style.textContent) return;
      let text = style.textContent;
      text = text.replace(/@font-face\s*\{[\s\S]*?\}/g, '');
      text = text.replace(/url\(([^)]+)\)/g, 'none');
      style.textContent = text;
    });

    clone.querySelectorAll('[style]').forEach((el) => {
      const inline = el.getAttribute('style');
      if (!inline || !inline.includes('url(')) return;
      const cleaned = inline.replace(/url\(([^)]+)\)/g, 'none');
      el.setAttribute('style', cleaned);
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
