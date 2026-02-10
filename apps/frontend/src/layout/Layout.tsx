import { ReactNode } from "react"

type LayoutProps = {
    children: ReactNode[];
}

export const Layout = ({ children }: LayoutProps) => {
    // children[0] = Canvas/Render
    // children[1] = Menu
    // children[2] = ConceptPanel (optional)

    const hasPanel = children.length >= 3;

    return (
        <div className="fixed inset-0 flex bg-slate-900">
            {/* Main content area */}
            <div className="flex-1 flex flex-col">
                {/* Canvas area - takes 70% height */}
                <div className="h-[70%] bg-slate-900">
                    {children[0]}
                </div>

                {/* Menu area - takes 30% height */}
                <div className="h-[30%] bg-slate-900 overflow-hidden">
                    {children[1]}
                </div>
            </div>

            {/* Concept Panel - fixed 320px width on the right */}
            {hasPanel && (
                <div className="w-80 flex-shrink-0 bg-terminal-bg">
                    {children[2]}
                </div>
            )}
        </div>
    )
}
