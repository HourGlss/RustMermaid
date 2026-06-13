const MIN_VIEWPORT_SCALE = 0.1;
const MAX_VIEWPORT_SCALE = 4;
const DEFAULT_FIT_PADDING = 30;
const DEFAULT_LOD_LABEL_SCALE = 0.65;
const DEFAULT_LOD_EDGE_SCALE = 0.5;
const DEFAULT_LOD_THRESHOLD = 250;
const DEFAULT_LOD_OVERSCAN_PX = 160;
const LOD_HIDDEN_CLASS = 'selkie-lod-hidden';

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

export function applyViewportLevelOfDetail(
    preview,
    viewport,
    renderParts,
    containerRect,
    options = {},
) {
    const stats = createLodStats();
    if (!preview || !renderParts) return stats;

    const nodes = renderParts.nodes ?? [];
    const edges = renderParts.edges ?? [];
    const isLargeGraph =
        nodes.length + edges.length >= (options.largeGraphThreshold ?? DEFAULT_LOD_THRESHOLD);
    const labelScale = options.labelDetailScale ?? DEFAULT_LOD_LABEL_SCALE;
    const edgeScale = options.edgeDetailScale ?? DEFAULT_LOD_EDGE_SCALE;
    const hideLabels = isLargeGraph && viewport.scale < labelScale;
    const cullEdges = isLargeGraph && viewport.scale < edgeScale;
    const visibleRect = cullEdges ? graphViewportRect(
        viewport,
        containerRect,
        options.overscanPx ?? DEFAULT_LOD_OVERSCAN_PX,
    ) : null;
    const nodeBounds = new Map(nodes.map((node) => [node.node_id, node.bounds]));
    const selectedNodeId = options.selectedNodeId ?? null;

    stats.enabled = isLargeGraph;
    stats.mode = hideLabels || cullEdges ? 'lod' : 'full';

    for (const node of nodes) {
        const nodeElement = nodeElementById(preview, node.node_id);
        if (!nodeElement) continue;

        const labelElements = nodeLabelElements(nodeElement);
        const isSelected = selectedNodeId === node.node_id;
        const showLabels = !hideLabels || isSelected;
        stats.totalNodeLabels += labelElements.length;

        for (const labelElement of labelElements) {
            setLodHidden(labelElement, !showLabels, stats);
            if (showLabels) {
                stats.visibleNodeLabels += 1;
            } else {
                stats.hiddenNodeLabels += 1;
            }
        }
    }

    for (const edge of edges) {
        const edgeElement = edgeElementById(preview, edge.edge_id);
        const edgeLabelElement = edgeLabelElementById(preview, edge.edge_id);
        const isIncidentToSelection =
            selectedNodeId && (edge.source === selectedNodeId || edge.target === selectedNodeId);
        const edgeIsVisible =
            !cullEdges || !visibleRect || edgeIntersectsViewport(edge, nodeBounds, visibleRect);
        const edgeLabelIsVisible = edgeIsVisible && (!hideLabels || isIncidentToSelection);

        stats.totalEdges += edgeElement ? 1 : 0;
        stats.totalEdgeLabels += edgeLabelElement ? 1 : 0;

        if (edgeElement) {
            setLodHidden(edgeElement, !edgeIsVisible, stats);
            if (edgeIsVisible) {
                stats.visibleEdges += 1;
            } else {
                stats.hiddenEdges += 1;
            }
        }

        if (edgeLabelElement) {
            setLodHidden(edgeLabelElement, !edgeLabelIsVisible, stats);
            if (edgeLabelIsVisible) {
                stats.visibleEdgeLabels += 1;
            } else {
                stats.hiddenEdgeLabels += 1;
            }
        }
    }

    stats.visibleDetailCount =
        stats.visibleNodeLabels + stats.visibleEdgeLabels + stats.visibleEdges;
    stats.renderPartUpdateCount = stats.visibleDetailCount;
    preview.dataset.selkieLodMode = stats.mode;
    preview.dataset.selkieVisibleDetailCount = String(stats.visibleDetailCount);
    return stats;
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

function createLodStats() {
    return {
        enabled: false,
        mode: 'full',
        totalNodeLabels: 0,
        visibleNodeLabels: 0,
        hiddenNodeLabels: 0,
        totalEdges: 0,
        visibleEdges: 0,
        hiddenEdges: 0,
        totalEdgeLabels: 0,
        visibleEdgeLabels: 0,
        hiddenEdgeLabels: 0,
        visibleDetailCount: 0,
        renderPartUpdateCount: 0,
        domUpdateCount: 0,
    };
}

function graphViewportRect(viewport, containerRect, overscanPx) {
    const scale = Math.max(viewport?.scale ?? 1, MIN_VIEWPORT_SCALE);
    const width = Number(containerRect?.width) || 0;
    const height = Number(containerRect?.height) || 0;
    if (!width || !height) return null;

    const overscan = Math.max(0, overscanPx) / scale;

    return {
        x: (0 - (viewport?.x ?? 0)) / scale - overscan,
        y: (0 - (viewport?.y ?? 0)) / scale - overscan,
        width: width / scale + overscan * 2,
        height: height / scale + overscan * 2,
    };
}

function nodeLabelElements(node) {
    return [...node.querySelectorAll('text, foreignObject')];
}

function edgeElementById(preview, edgeId) {
    return preview.querySelector(`#${cssEscape(`edge-${edgeId}`)}`);
}

function edgeLabelElementById(preview, edgeId) {
    return preview.querySelector(`#${cssEscape(`edge-label-${edgeId}`)}`);
}

function edgeIntersectsViewport(edge, nodeBounds, visibleRect) {
    const bounds = mergedBounds([
        nodeBounds.get(edge.source),
        nodeBounds.get(edge.target),
        pointBounds(edge.points),
    ]);
    return !bounds || rectsIntersect(expandRect(bounds, 40), visibleRect);
}

function mergedBounds(boundsList) {
    const bounds = boundsList.filter(Boolean);
    if (!bounds.length) return null;

    const minX = Math.min(...bounds.map((rect) => rect.x));
    const minY = Math.min(...bounds.map((rect) => rect.y));
    const maxX = Math.max(...bounds.map((rect) => rect.x + rect.width));
    const maxY = Math.max(...bounds.map((rect) => rect.y + rect.height));
    return {
        x: minX,
        y: minY,
        width: maxX - minX,
        height: maxY - minY,
    };
}

function pointBounds(points) {
    if (!points?.length) return null;

    const xs = points.map((point) => point.x);
    const ys = points.map((point) => point.y);
    const minX = Math.min(...xs);
    const minY = Math.min(...ys);
    return {
        x: minX,
        y: minY,
        width: Math.max(...xs) - minX,
        height: Math.max(...ys) - minY,
    };
}

function expandRect(rect, amount) {
    return {
        x: rect.x - amount,
        y: rect.y - amount,
        width: rect.width + amount * 2,
        height: rect.height + amount * 2,
    };
}

function rectsIntersect(a, b) {
    return (
        a.x <= b.x + b.width &&
        a.x + a.width >= b.x &&
        a.y <= b.y + b.height &&
        a.y + a.height >= b.y
    );
}

function setLodHidden(element, hidden, stats) {
    const wasHidden = element.classList.contains(LOD_HIDDEN_CLASS);
    element.classList.toggle(LOD_HIDDEN_CLASS, hidden);
    element.style.display = hidden ? 'none' : '';
    element.setAttribute('aria-hidden', hidden ? 'true' : 'false');

    if (wasHidden !== hidden) {
        stats.domUpdateCount += 1;
    }
}

function cssEscape(value) {
    if (globalThis.CSS?.escape) {
        return globalThis.CSS.escape(value);
    }

    return String(value).replace(/["\\#.;:[\],>+~*^$|=(){}\s]/g, '\\$&');
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
