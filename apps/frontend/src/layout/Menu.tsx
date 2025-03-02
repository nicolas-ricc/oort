import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command"


export const Menu = ({ concepts, onSelect }) => {
    return (
            <Command>

                <CommandInput placeholder="Type a command or search..." />

                <CommandList className="max-h-64  overflow-hidden">
                    <CommandEmpty>No results found.</CommandEmpty>
                    <CommandGroup>

                        {concepts.map((concept, index) => (
                            <CommandItem onSelect={() => onSelect(index)} key={index}>
                                <span className="uppercase">{concept}</span>
                            </CommandItem>
                        ))}
                    </CommandGroup>

                </CommandList>
            </Command>


    )
}