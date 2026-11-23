import '@testing-library/jest-dom/vitest';

// jsdom não tem ResizeObserver nem canvas 2D: o visor funciona sem eles (não mede nem desenha).
class ResizeObserverDeTeste {
  observados = 0;
  observe() {
    this.observados += 1;
  }
  unobserve() {
    this.observados -= 1;
  }
  disconnect() {
    this.observados = 0;
  }
}
globalThis.ResizeObserver = ResizeObserverDeTeste;
HTMLCanvasElement.prototype.getContext = () => null;
