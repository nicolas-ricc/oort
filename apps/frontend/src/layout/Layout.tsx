import { ReactNode, RefObject } from "react"

type LayoutProps = {
    children: ReactNode[];
    isEmpty?: boolean;
    canvasRef?: RefObject<HTMLDivElement | null>;
}

export const Layout = ({ children, isEmpty, canvasRef }: LayoutProps) => {
    // children[0] = Canvas/Render
    // children[1] = Menu

    return (
        <div className="fixed inset-0 flex bg-slate-900">
            <div className="flex-1 flex flex-col">
                {/* Canvas area */}
                <div
                    ref={canvasRef}
                    className={`${isEmpty ? 'h-full' : 'h-[70%]'} bg-slate-900 relative`}
                >
                    {children[0]}
                </div>

                {/* Menu area */}
                {!isEmpty && (
                    <div className="h-[30%] bg-slate-900 overflow-hidden">
                        {children[1]}
                    </div>
                )}
            </div>
        </div>
    )
}
