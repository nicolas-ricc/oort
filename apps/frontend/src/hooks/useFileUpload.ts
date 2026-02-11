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
            if (!file) return;
            const reader = new FileReader();
            const handleFileUpload = async (file: File, reader: FileReader) => {
                return new Promise((resolve) => {
                    reader.onload = async (e) => {
                        const text = e.target?.result as string;
                        resolve(text)
                    }
                    reader.readAsText(file);
                })
            }
            await handleFileUpload(file, reader)
            const res = await fetch('http://localhost:8000/api/vectorize', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    "Accept-Cross-Origin": "*",
                    "Access-Control-Allow-Origin": "*",
                },
                body: JSON.stringify({ user_id: defaultUserId, text: reader.result, filename: file.name }),
            }).then(async (res) => res.json()).catch(err => console.error(err))
            const vectors = res.data
            return vectors
        },
        onMutate: () => {
            setLoadingState(true)
        },
        onSuccess: (data) => {
            onSimulationUpdate(data)
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
