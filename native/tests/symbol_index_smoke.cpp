#include "../src/symbol_index.hpp"

#include <QString>

int main()
{
    const QString source = QStringLiteral(
        "import os\n"
        "class Workspace:\n"
        "    async def start(self):\n"
        "        return True\n");
    const QList<SourceSymbol> symbols = SymbolIndex::scan(source, QStringLiteral("py"));
    if (symbols.size() != 2) return 1;
    if (symbols.at(0).name != QStringLiteral("Workspace")
        || symbols.at(0).kind != QStringLiteral("clase") || symbols.at(0).line != 2) return 2;
    if (symbols.at(1).name != QStringLiteral("start")
        || symbols.at(1).kind != QStringLiteral("función") || symbols.at(1).line != 3) return 3;

    const QList<SourceSymbol> scripts = SymbolIndex::scan(
        QStringLiteral("export interface Project { }\nconst load = async () => {}\n"),
        QStringLiteral("ts"));
    if (scripts.size() != 2 || scripts.at(0).name != QStringLiteral("Project")
        || scripts.at(1).name != QStringLiteral("load")) return 4;
    if (!SymbolIndex::scan(source, QStringLiteral("md")).isEmpty()) return 5;
    const QList<SourceSymbol> native = SymbolIndex::scan(
        QStringLiteral("class Runner {\npublic:\n    int run() { return 0; }\n};\n"),
        QStringLiteral("cpp"));
    if (native.size() != 2 || native.at(0).name != QStringLiteral("Runner")
        || native.at(1).name != QStringLiteral("run")) return 6;
    return 0;
}
