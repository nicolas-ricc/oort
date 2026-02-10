import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Menu } from './Menu'
import { renderWithProviders } from '@/test/test-utils'

// Mock data
const mockConcepts = [
  {
    concepts: ['machine learning', 'neural networks'],
    reduced_embedding: [1.0, 2.0, 3.0],
    cluster_id: 0,
  },
  {
    concepts: ['data science'],
    reduced_embedding: [4.0, 5.0, 6.0],
    cluster_id: 1,
  },
]

const mockApiResponse = {
  success: true,
  data: mockConcepts,
}

describe('Menu', () => {
  const mockOnSelect = vi.fn()
  const mockOnSimulationUpdate = vi.fn()
  const mockSetLoadingState = vi.fn()

  const defaultProps = {
    concepts: [],
    onSelect: mockOnSelect,
    active: '',
    onSimulationUpdate: mockOnSimulationUpdate,
    setLoadingState: mockSetLoadingState,
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.resetAllMocks()
  })

  describe('Unit Tests', () => {
    it('renders the file upload input', () => {
      renderWithProviders(<Menu {...defaultProps} />)

      const fileInput = document.getElementById('file-upload') as HTMLInputElement
      expect(fileInput).toBeInTheDocument()
      expect(fileInput.type).toBe('file')
      expect(fileInput.accept).toBe('.txt,.md,.text')
    })

    it('renders concept list when concepts are provided', () => {
      renderWithProviders(<Menu {...defaultProps} concepts={mockConcepts} />)

      expect(screen.getByText('machine learning')).toBeInTheDocument()
      expect(screen.getByText('neural networks')).toBeInTheDocument()
      expect(screen.getByText('data science')).toBeInTheDocument()
    })

    it('calls onSelect when a concept is clicked', async () => {
      const user = userEvent.setup()
      renderWithProviders(<Menu {...defaultProps} concepts={mockConcepts} />)

      await user.click(screen.getByText('machine learning'))

      expect(mockOnSelect).toHaveBeenCalledWith('machine learning')
    })

    it('shows search input placeholder', () => {
      renderWithProviders(<Menu {...defaultProps} />)

      expect(screen.getByPlaceholderText('Search concepts...')).toBeInTheDocument()
    })

    it('shows "No results found" for empty concepts list', () => {
      renderWithProviders(<Menu {...defaultProps} concepts={[]} />)

      // CommandEmpty only shows when searching with no matches
      // With empty concepts, it should still be in the DOM
      expect(screen.getByText('No results found.')).toBeInTheDocument()
    })
  })

  describe('File Upload - Loading State', () => {
    it('calls setLoadingState(true) when file upload starts', async () => {
      const user = userEvent.setup()
      vi.mocked(fetch).mockResolvedValue({
        json: () => Promise.resolve(mockApiResponse),
      } as Response)

      renderWithProviders(<Menu {...defaultProps} />)

      const file = new File(['test content'], 'test.txt', { type: 'text/plain' })
      const fileInput = document.getElementById('file-upload') as HTMLInputElement

      await user.upload(fileInput, file)

      expect(mockSetLoadingState).toHaveBeenCalledWith(true)
    })

    it('calls setLoadingState(false) after file upload completes', async () => {
      const user = userEvent.setup()
      vi.mocked(fetch).mockResolvedValue({
        json: () => Promise.resolve(mockApiResponse),
      } as Response)

      renderWithProviders(<Menu {...defaultProps} />)

      const file = new File(['test content'], 'test.txt', { type: 'text/plain' })
      const fileInput = document.getElementById('file-upload') as HTMLInputElement

      await user.upload(fileInput, file)

      await waitFor(() => {
        expect(mockSetLoadingState).toHaveBeenCalledWith(false)
      })
    })

    it('calls setLoadingState(false) even when upload fails', async () => {
      const user = userEvent.setup()
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
      vi.mocked(fetch).mockRejectedValue(new Error('Network error'))

      renderWithProviders(<Menu {...defaultProps} />)

      const file = new File(['test content'], 'test.txt', { type: 'text/plain' })
      const fileInput = document.getElementById('file-upload') as HTMLInputElement

      await user.upload(fileInput, file)

      await waitFor(() => {
        expect(mockSetLoadingState).toHaveBeenCalledWith(false)
      })

      consoleErrorSpy.mockRestore()
    })
  })

  describe('File Upload - API Integration', () => {
    it('sends correct payload to API on file upload', async () => {
      const user = userEvent.setup()
      vi.mocked(fetch).mockResolvedValue({
        json: () => Promise.resolve(mockApiResponse),
      } as Response)

      renderWithProviders(<Menu {...defaultProps} />)

      const fileContent = 'This is test content for vectorization'
      const file = new File([fileContent], 'document.txt', { type: 'text/plain' })
      const fileInput = document.getElementById('file-upload') as HTMLInputElement

      await user.upload(fileInput, file)

      await waitFor(() => {
        expect(fetch).toHaveBeenCalledWith(
          'http://localhost:8000/api/vectorize',
          expect.objectContaining({
            method: 'POST',
            headers: expect.objectContaining({
              'Content-Type': 'application/json',
            }),
            body: expect.any(String),
          })
        )
      })

      // Verify the body contains expected fields
      const fetchCall = vi.mocked(fetch).mock.calls[0]
      const body = JSON.parse(fetchCall[1]?.body as string)
      expect(body).toMatchObject({
        user_id: '550e8400-e29b-41d4-a716-446655440000',
        text: fileContent,
        filename: 'document.txt',
      })
    })

    it('calls onSimulationUpdate with API response data on success', async () => {
      const user = userEvent.setup()
      vi.mocked(fetch).mockResolvedValue({
        json: () => Promise.resolve(mockApiResponse),
      } as Response)

      renderWithProviders(<Menu {...defaultProps} />)

      const file = new File(['test content'], 'test.txt', { type: 'text/plain' })
      const fileInput = document.getElementById('file-upload') as HTMLInputElement

      await user.upload(fileInput, file)

      await waitFor(() => {
        expect(mockOnSimulationUpdate).toHaveBeenCalledWith(mockConcepts)
      })
    })

    it('shows alert and logs error when API call fails', async () => {
      const user = userEvent.setup()
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
      const alertSpy = vi.mocked(window.alert)

      // Simulate network failure
      vi.mocked(fetch).mockRejectedValue(new Error('Network error'))

      renderWithProviders(<Menu {...defaultProps} />)

      const file = new File(['test content'], 'test.txt', { type: 'text/plain' })
      const fileInput = document.getElementById('file-upload') as HTMLInputElement

      await user.upload(fileInput, file)

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalled()
        expect(alertSpy).toHaveBeenCalledWith('Error processing file. Please try again.')
      })

      consoleErrorSpy.mockRestore()
    })

    it('does not make API call when no file is selected', async () => {
      renderWithProviders(<Menu {...defaultProps} />)

      const fileInput = document.getElementById('file-upload') as HTMLInputElement

      // Simulate empty file selection (user cancels file dialog)
      const emptyEvent = new Event('change', { bubbles: true })
      Object.defineProperty(emptyEvent, 'target', {
        value: { files: [] },
      })
      fileInput.dispatchEvent(emptyEvent)

      // Wait a bit to ensure no async operations occurred
      await new Promise(resolve => setTimeout(resolve, 50))

      expect(fetch).not.toHaveBeenCalled()
    })
  })

  describe('File Input State', () => {
    it('file input is not disabled by default', () => {
      renderWithProviders(<Menu {...defaultProps} />)

      const fileInput = document.getElementById('file-upload') as HTMLInputElement
      expect(fileInput.disabled).toBe(false)
    })
  })
})
