const MIN_VIEWPORT_SCALE = 0.1;
const MAX_VIEWPORT_SCALE = 4;
const DEFAULT_FIT_PADDING = 30;

export function createViewportState() {
    return {
        scale: 1,
        x: 0,
        y: 0,
    };
}

export function applyViewportTransform(preview, viewport, zoomLabel = null) {
    if (!preview) return;

    preview.style.transform =
        `translate(${formatPx(viewport.x)}, ${formatPx(viewport.y)}) scale(${formatNumber(viewport.scale)})`;

    if (zoomLabel) {
        zoomLabel.textContent = `${Math.round(viewport.scale * 100)}%`;
    }
}

export function zoomViewportBy(viewport, factor, origin = { x: 0, y: 0 }) {
    const previousScale = viewport.scale;
    const nextScale = clamp(previousScale * factor, MIN_VIEWPORT_SCALE, MAX_VIEWPORT_SCALE);

    if (nextScale === previousScale) {
        return viewport;
    }

    const scaleRatio = nextScale / previousScale;
    viewport.x = origin.x - (origin.x - viewport.x) * scaleRatio;
    viewport.y = origin.y - (origin.y - viewport.y) * scaleRatio;
    viewport.scale = nextScale;
    return viewport;
}

export function panViewportBy(viewport, dx, dy) {
    viewport.x += dx;
    viewport.y += dy;
    return viewport;
}

export function resetViewport(viewport) {
    viewport.scale = 1;
    viewport.x = 0;
    viewport.y = 0;
    return viewport;
}

export function fitViewportToSvg(
    viewport,
    containerRect,
    svgRect,
    padding = DEFAULT_FIT_PADDING,
) {
    if (
        !containerRect?.width ||
        !containerRect?.height ||
        !svgRect?.width ||
        !svgRect?.height
    ) {
        return resetViewport(viewport);
    }

    const availableWidth = Math.max(1, containerRect.width - padding * 2);
    const availableHeight = Math.max(1, containerRect.height - padding * 2);
    const scale = clamp(
        Math.min(availableWidth / svgRect.width, availableHeight / svgRect.height),
        MIN_VIEWPORT_SCALE,
        MAX_VIEWPORT_SCALE,
    );

    viewport.scale = scale;
    viewport.x = (containerRect.width - svgRect.width * scale) / 2;
    viewport.y = (containerRect.height - svgRect.height * scale) / 2;
    return viewport;
}

export function installNodeDrag(preview, options = {}) {
    if (!preview) return () => {};

    const doc = preview.ownerDocument;
    const getScale = options.getScale ?? (() => 1);
    let drag = null;

    function onMouseDown(event) {
        const node = closestNodeElement(event.target);
        if (!node) return;

        drag = {
            node,
            id: nodeIdFromElement(node),
            startX: event.clientX,
            startY: event.clientY,
            dx: 0,
            dy: 0,
        };

        node.classList.add('is-dragging');
        event.preventDefault();
    }

    function onMouseMove(event) {
        if (!drag) return;

        const scale = Math.max(getScale(), MIN_VIEWPORT_SCALE);
        drag.dx = (event.clientX - drag.startX) / scale;
        drag.dy = (event.clientY - drag.startY) / scale;
        drag.node.style.transform = `translate(${formatPx(drag.dx)}, ${formatPx(drag.dy)})`;
        drag.node.dataset.selkieDragging = 'true';

        options.onDrag?.({
            id: drag.id,
            dx: drag.dx,
            dy: drag.dy,
            node: drag.node,
        });
    }

    function onMouseUp() {
        if (!drag) return;

        drag.node.classList.remove('is-dragging');
        options.onCommit?.({
            id: drag.id,
            dx: drag.dx,
            dy: drag.dy,
            node: drag.node,
        });
        drag = null;
    }

    preview.addEventListener('mousedown', onMouseDown);
    doc.addEventListener('mousemove', onMouseMove);
    doc.addEventListener('mouseup', onMouseUp);

    return () => {
        preview.removeEventListener('mousedown', onMouseDown);
        doc.removeEventListener('mousemove', onMouseMove);
        doc.removeEventListener('mouseup', onMouseUp);
    };
}

export function installNodeSelection(preview, onSelect) {
    if (!preview) return () => {};

    function onClick(event) {
        const node = closestNodeElement(event.target);
        if (!node) return;

        onSelect?.({
            id: nodeIdFromElement(node),
            node,
            event,
        });
    }

    preview.addEventListener('click', onClick);
    return () => preview.removeEventListener('click', onClick);
}

export function markSelectedNode(preview, nodeId) {
    if (!preview) return null;

    let selected = null;
    preview.querySelectorAll('[id^="node-"]').forEach((node) => {
        const isSelected = nodeIdFromElement(node) === nodeId;
        node.classList.toggle('is-selected', isSelected);
        if (isSelected) {
            selected = node;
        }
    });
    return selected;
}

export function applyNodeVisualEdit(preview, nodeId, edit) {
    const node = nodeElementById(preview, nodeId);
    if (!node) return null;

    if (edit.label !== undefined) {
        const text = node.querySelector('text');
        if (text) {
            text.textContent = edit.label;
        }
    }

    if (edit.color) {
        const shape = node.querySelector('rect,path,polygon,ellipse,circle');
        if (shape) {
            shape.setAttribute('fill', edit.color);
            shape.style.fill = edit.color;
        }
    }

    return node;
}

export function installViewportPan(container, viewport, onChange, shouldIgnoreTarget) {
    if (!container) return () => {};

    const doc = container.ownerDocument;
    let pan = null;

    function onMouseDown(event) {
        if (event.button !== 0 || shouldIgnoreTarget?.(event.target)) return;

        pan = {
            startX: event.clientX,
            startY: event.clientY,
        };
        container.classList.add('is-panning');
        event.preventDefault();
    }

    function onMouseMove(event) {
        if (!pan) return;

        const dx = event.clientX - pan.startX;
        const dy = event.clientY - pan.startY;
        pan.startX = event.clientX;
        pan.startY = event.clientY;
        panViewportBy(viewport, dx, dy);
        onChange?.();
    }

    function onMouseUp() {
        if (!pan) return;
        pan = null;
        container.classList.remove('is-panning');
    }

    container.addEventListener('mousedown', onMouseDown);
    doc.addEventListener('mousemove', onMouseMove);
    doc.addEventListener('mouseup', onMouseUp);

    return () => {
        container.removeEventListener('mousedown', onMouseDown);
        doc.removeEventListener('mousemove', onMouseMove);
        doc.removeEventListener('mouseup', onMouseUp);
    };
}

export function closestNodeElement(target) {
    if (!target?.closest) return null;
    return target.closest('[id^="node-"]');
}

export function nodeIdFromElement(node) {
    return node.id.replace(/^node-/, '');
}

export function nodeElementById(preview, nodeId) {
    if (!preview) return null;

    return [...preview.querySelectorAll('[id^="node-"]')]
        .find((node) => nodeIdFromElement(node) === nodeId) ?? null;
}

function clamp(value, min, max) {
    return Math.min(Math.max(value, min), max);
}

function formatPx(value) {
    return `${formatNumber(value)}px`;
}

function formatNumber(value) {
    if (Number.isInteger(value)) {
        return String(value);
    }

    return String(Number(value.toFixed(4)));
}
