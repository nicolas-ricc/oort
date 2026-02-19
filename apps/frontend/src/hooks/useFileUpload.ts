import { ChangeEvent } from "react"
import { useMutation } from "@tanstack/react-query"

const defaultUserId = "550e8400-e29b-41d4-a716-446655440000";

type UseFileUploadOptions = {
    onSimulationUpdate: (data: any) => void;
    setLoadingState: (loading: boolean) => void;
}

export function useFileUpload({ onSimulationUpdate, setLoadingState }: UseFileUploadOptions) {
    const { mutate: uploadFile, isPending } = useMutation({
        mutationFn: async (event: ChangeEvent<HTMLInputElement>) => {
            const [file] = event.target.files || [];
            if (!file) return null;

            const text = await new Promise<string>((resolve, reject) => {
                const reader = new FileReader();
                reader.onload = (e) => resolve(e.target?.result as string);
                reader.onerror = () => reject(new Error('Failed to read file'));
                reader.readAsText(file);
            });

            const res = await fetch('http://localhost:8000/api/vectorize', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    "Accept-Cross-Origin": "*",
                    "Access-Control-Allow-Origin": "*",
                },
                body: JSON.stringify({ user_id: defaultUserId, text, filename: file.name }),
            });

            if (!res.ok) {
                const body = await res.json().catch(() => null);
                const detail = body?.detail || `Server error (${res.status})`;
                throw new Error(detail);
            }

            const data = await res.json();
            return data.data;
        },
        onMutate: () => {
            setLoadingState(true)
        },
        onSuccess: (data) => {
            if (data) {
                onSimulationUpdate(data)
            }
        },
        onError: (error) => {
            console.error('Error processing file:', error);
            alert('Error processing file. Please try again.');
        },
        onSettled: () => {
            setLoadingState(false)
        }
    })

    return { uploadFile, isPending }
}
