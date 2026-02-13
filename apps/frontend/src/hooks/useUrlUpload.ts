import { useMutation } from "@tanstack/react-query"

const defaultUserId = "550e8400-e29b-41d4-a716-446655440000";

type UseUrlUploadOptions = {
    onSimulationUpdate: (data: any) => void;
    setLoadingState: (loading: boolean) => void;
}

export function useUrlUpload({ onSimulationUpdate, setLoadingState }: UseUrlUploadOptions) {
    const { mutate: uploadUrl, isPending } = useMutation({
        mutationFn: async (url: string) => {
            const res = await fetch('http://localhost:8000/api/vectorize', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    "Accept-Cross-Origin": "*",
                    "Access-Control-Allow-Origin": "*",
                },
                body: JSON.stringify({ user_id: defaultUserId, url }),
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
            onSimulationUpdate(data)
        },
        onError: (error) => {
            console.error('Error processing URL:', error);
            alert(error.message || 'Error processing URL. Please try again.');
        },
        onSettled: () => {
            setLoadingState(false)
        }
    })

    return { uploadUrl, isPending }
}
