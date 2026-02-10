import '@testing-library/jest-dom'
import { vi } from 'vitest'

// Mock window.alert globally
vi.stubGlobal('alert', vi.fn())

// Mock fetch globally - individual tests can override with mockResolvedValue/mockRejectedValue
vi.stubGlobal('fetch', vi.fn())

// Mock ResizeObserver (not available in jsdom)
class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal('ResizeObserver', ResizeObserverMock)

// Mock scrollIntoView (not available in jsdom, used by cmdk)
Element.prototype.scrollIntoView = vi.fn()
