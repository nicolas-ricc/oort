import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command"
import { cn } from "@/lib/utils"
import { Upload } from "lucide-react"


export const Menu = ({ concepts, onSelect }) => {
    return (
        <div className=" [>*]:text-[32px] bg-terminal-bg border-2 border-terminal-border shadow-lg overflow-hidden relative h-full mx-auto">
            <div className="relative h-full">

                <div className="absolute inset-0 pointer-events-none z-10 bg-[repeating-linear-gradient(0deg,rgba(0,0,0,0.15),rgba(0,0,0,0.15)_1px,transparent_1px,transparent_2px)]"></div>
                <div className="absolute inset-0 pointer-events-none z-20 bg-[radial-gradient(ellipse_at_center,transparent_0%,rgba(0,0,0,0.2)_90%,rgba(0,0,0,0.4)_100%)]"></div>

                <Command className="rounded-none border-none bg-transparent">
                    <div className="border-y border-terminal-border px-3 py-1 bg-terminal-bg">
                        <CommandInput className="border-none outline-none text-terminal-text bg-transparent placeholder-terminal-muted caret-terminal-text text-[16px]"
                            placeholder="What are you looking for..." />
                        <label htmlFor="file-upload" className={`cursor-pointer flex items-center justify-center w-10 h-10 rounded-md transition-colors ${false ? 'text-gray-500' : 'text-terminal-text hover:text-green-300 hover:bg-zinc-800'}`}>
                            <Upload size={20} />
                            <input
                                id="file-upload"
                                type="file"
                                accept=".txt,.md,.text"
                                className="hidden"
/*                                 onChange={handleFileUpload}
                                disabled={isUploading} */
                            />
                        </label>
                    </div>

                    <CommandList className="max-h-80 overflow-y-auto scrollbar scrollbar-w-3 scrollbar-thumb-terminal-border scrollbar-track-zinc-800">
                        <CommandEmpty className="px-4 py-8 text-center text-terminal-muted">No results found.</CommandEmpty>
                        <CommandGroup className="bg-terminal-bg text-terminal-text">

                            {concepts.map((concept: string, index: number) => (
                                <CommandItem onSelect={() => onSelect(index)} key={concept} className={cn(
                                    "px-4 py-3 cursor-pointer transition-colors text-[32px]",
                                    false
                                        ? " "
                                        : "text-terminal-text hover:bg-zinc-800 hover:text-green-300",
                                )}>
                                    <span >{concept}</span>
                                </CommandItem>
                            ))}
                        </CommandGroup>

                    </CommandList>
                    <div className="bg-zinc-800 border-t border-terminal-border px-4 py-2 text-xs text-terminal-text flex justify-between items-center">
                        <div className="flex items-center">
                            <span className="inline-block w-2 h-2 bg-terminal-text mr-2 animate-terminal-blink"></span>
                            OORT TERMINAL v1.0.0
                        </div>
                        <div className="flex gap-4">
                            <span>↑↓: NAVIGATE</span>
                            <span>ENTER: SELECT</span>
                        </div>
                    </div>
                </Command>
            </div>

        </div>



    )
}