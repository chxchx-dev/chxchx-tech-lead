#include "../src/file_icon_provider.hpp"
#include "../src/responsive_grid.hpp"

#include <QApplication>
#include <QGridLayout>
#include <QPushButton>

namespace {

int rowOf(QWidget *grid, QWidget *widget)
{
    auto *layout = static_cast<QGridLayout *>(grid->layout());
    const int index = layout->indexOf(widget);
    if (index < 0) return -1;
    int row = 0, column = 0, rowSpan = 0, columnSpan = 0;
    layout->getItemPosition(index, &row, &column, &rowSpan, &columnSpan);
    return row;
}

void settle()
{
    for (int i = 0; i < 5; ++i) QApplication::processEvents();
}

} // namespace

int main(int argc, char **argv)
{
    QApplication app(argc, argv);

    ResponsiveGrid grid(200, 4);
    auto *selector = new QPushButton(QStringLiteral("selector"));
    grid.addWidget(selector, true);
    QList<QPushButton *> buttons;
    for (int index = 0; index < 4; ++index) {
        buttons << new QPushButton(QStringLiteral("acción %1").arg(index));
        grid.addWidget(buttons.last());
    }
    grid.resize(1000, 200);
    grid.show();
    settle();
    if (grid.columns() != 4 || rowOf(&grid, buttons.at(3)) != 1) return 1;

    grid.resize(450, 200);
    settle();
    if (grid.columns() != 2 || rowOf(&grid, buttons.at(3)) != 2) return 2;

    // Hidden widgets leave no gap: the next visible one takes their place.
    buttons.at(1)->hide();
    settle();
    if (rowOf(&grid, buttons.at(1)) != -1 || rowOf(&grid, buttons.at(2)) != 1) return 3;
    for (QPushButton *button : buttons) button->hide();
    selector->hide();
    settle();
    if (grid.hasVisibleItems()) return 4;

    grid.resize(120, 200);
    settle();
    if (grid.columns() != 1) return 5;

    if (studioFileKind(QStringLiteral("src"), true) != QStringLiteral("folder")) return 10;
    if (studioFileKind(QStringLiteral("main.cpp"), false) != QStringLiteral("code")) return 11;
    if (studioFileKind(QStringLiteral("README.md"), false) != QStringLiteral("doc")) return 12;
    if (studioFileKind(QStringLiteral("pyproject.toml"), false) != QStringLiteral("config")) return 13;
    if (studioFileKind(QStringLiteral("CMakeLists.txt"), false) != QStringLiteral("build")) return 14;
    if (studioFileKind(QStringLiteral("logo.PNG"), false) != QStringLiteral("image")) return 15;
    if (studioFileKind(QStringLiteral(".gitignore"), false) != QStringLiteral("dotfile")) return 16;
    if (studioFileKind(QStringLiteral("LICENSE"), false) != QStringLiteral("file")) return 17;
    return 0;
}
