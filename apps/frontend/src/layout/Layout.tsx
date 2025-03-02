export const Layout = ({ children }) => {
    return (
        <div className="fixed inset-0 flex flex-col  bg-slate-900">
            <div className="h-2/3 bg-slate-900">
                {children[0]}
            </div>

            <div className="h-1/3 bg-slate-900  overflow-hidden ">
                {children[1]}
            </div>
        </div>
    )
}