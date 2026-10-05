cmake_minimum_required(VERSION 3.21)

project(chxchx_studio VERSION 0.1.0 LANGUAGES C CXX)

option(CHXCHX_BUILD_TESTS "Build native editor smoke tests" OFF)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 6.4 REQUIRED COMPONENTS Core Widgets Core5Compat)

include(FetchContent)
FetchContent_Declare(
    scintilla
    GIT_REPOSITORY https://github.com/mirror/scintilla.git
    GIT_TAG a1c86144eed9e3d2187e3a8b391d11ca909f00d2
    GIT_PROGRESS TRUE
    SOURCE_SUBDIR cmake
)
FetchContent_Declare(
    lexilla
    GIT_REPOSITORY https://github.com/ScintillaOrg/lexilla.git
    GIT_TAG 8fe5438b65cffc34340ecd60357ce5a6f72eecbe
    GIT_PROGRESS TRUE
    SOURCE_SUBDIR cmake
)
FetchContent_MakeAvailable(scintilla lexilla)

FetchContent_Declare(
    libvterm
    GIT_REPOSITORY https://github.com/neovim/libvterm.git
    GIT_TAG v0.3.3
    GIT_SHALLOW TRUE
)
FetchContent_MakeAvailable(libvterm)
find_package(Perl REQUIRED)
file(GLOB LIBVTERM_TABLES "${CMAKE_BINARY_DIR}/_deps/libvterm-src/src/encoding/*.tbl")
foreach(table IN LISTS LIBVTERM_TABLES)
    get_filename_component(table_name "${table}" NAME_WE)
    execute_process(
        COMMAND "${PERL_EXECUTABLE}" -CSD "${CMAKE_BINARY_DIR}/_deps/libvterm-src/tbl2inc_c.pl" "${table}"
        OUTPUT_FILE "${CMAKE_BINARY_DIR}/_deps/libvterm-src/src/encoding/${table_name}.inc"
        RESULT_VARIABLE table_result
    )
    if(NOT table_result EQUAL 0)
        message(FATAL_ERROR "Could not generate libvterm encoding table: ${table}")
    endif()
endforeach()
file(GLOB LIBVTERM_SOURCES CONFIGURE_DEPENDS "${CMAKE_BINARY_DIR}/_deps/libvterm-src/src/*.c")
add_library(chxchx_vterm STATIC ${LIBVTERM_SOURCES})
set_target_properties(chxchx_vterm PROPERTIES AUTOMOC OFF)
target_include_directories(chxchx_vterm PUBLIC "${CMAKE_BINARY_DIR}/_deps/libvterm-src/include")

file(GLOB SCINTILLA_CORE_SOURCES CONFIGURE_DEPENDS "${scintilla_SOURCE_DIR}/src/*.cxx")
add_library(chxchx_scintilla STATIC
    ${SCINTILLA_CORE_SOURCES}
    "${scintilla_SOURCE_DIR}/qt/ScintillaEditBase/PlatQt.cpp"
    "${scintilla_SOURCE_DIR}/qt/ScintillaEditBase/ScintillaQt.cpp"
    "${scintilla_SOURCE_DIR}/qt/ScintillaEditBase/ScintillaEditBase.cpp"
    "${scintilla_SOURCE_DIR}/qt/ScintillaEditBase/ScintillaEditBase.h"
    "${scintilla_SOURCE_DIR}/qt/ScintillaEditBase/ScintillaQt.h"
)
target_include_directories(chxchx_scintilla PUBLIC
    "${scintilla_SOURCE_DIR}/include"
    "${scintilla_SOURCE_DIR}/src"
    "${scintilla_SOURCE_DIR}/qt/ScintillaEditBase"
)
target_compile_definitions(chxchx_scintilla PRIVATE SCINTILLA_QT=1 MAKING_LIBRARY=1 NDEBUG=1)
target_precompile_headers(chxchx_scintilla PRIVATE <cstdint>)
target_link_libraries(chxchx_scintilla PUBLIC Qt6::Core Qt6::Gui Qt6::Widgets Qt6::Core5Compat)

file(GLOB LEXILLA_SOURCES CONFIGURE_DEPENDS
    "${lexilla_SOURCE_DIR}/src/*.cxx"
    "${lexilla_SOURCE_DIR}/lexlib/*.cxx"
    "${lexilla_SOURCE_DIR}/lexers/*.cxx"
)
add_library(chxchx_lexilla STATIC ${LEXILLA_SOURCES})
target_include_directories(chxchx_lexilla PUBLIC
    "${lexilla_SOURCE_DIR}/include"
    "${lexilla_SOURCE_DIR}/lexlib"
    "${scintilla_SOURCE_DIR}/include"
)
target_compile_definitions(chxchx_lexilla PRIVATE LEXILLA_NO_EXPORT NDEBUG=1)

qt_add_executable(chxchx-studio
    src/main.cpp
    src/main_window.cpp
    src/main_window_area_controller.cpp
    src/main_window_actions.cpp
    src/main_window_agent_sessions.cpp
    src/main_window_data_pages.cpp
    src/main_window_layout.cpp
    src/main_window_status_presenter.cpp
    src/main_window.hpp
    src/main_window_areas.hpp
    src/integrations/terminal_launcher.cpp
    src/integrations/terminal_launcher.hpp
    src/integrations/bridge_client.cpp
    src/integrations/bridge_client.hpp
    src/integrations/bridge_schemas.hpp
    src/integrations/agent_session_widget.cpp
    src/integrations/agent_session_widget.hpp
    src/integrations/vt_terminal_widget.cpp
    src/integrations/vt_terminal_widget.hpp
    src/integrations/pty_session.cpp
    src/integrations/pty_session.hpp
)

target_link_libraries(chxchx-studio PRIVATE Qt6::Core Qt6::Widgets Qt6::Core5Compat
    chxchx_scintilla chxchx_lexilla chxchx_vterm)
target_compile_definitions(chxchx-studio PRIVATE QT_NO_CAST_FROM_ASCII QT_NO_CAST_TO_ASCII)
if(WIN32)
    target_compile_definitions(chxchx-studio PRIVATE WINVER=0x0A00 _WIN32_WINNT=0x0A00)
elseif(UNIX AND NOT APPLE)
    find_library(CHXCHX_UTIL_LIBRARY util)
    if(CHXCHX_UTIL_LIBRARY)
        target_link_libraries(chxchx-studio PRIVATE ${CHXCHX_UTIL_LIBRARY})
    endif()
endif()

if(MSVC)
    target_compile_options(chxchx-studio PRIVATE /W4 /permissive-)
else()
    target_compile_options(chxchx-studio PRIVATE -Wall -Wextra -Wpedantic)
endif()

install(TARGETS chxchx-studio
    BUNDLE DESTINATION .
    RUNTIME DESTINATION bin
)

if(CHXCHX_BUILD_TESTS)
    enable_testing()
    add_executable(chxchx-editor-smoke tests/editor_smoke.cpp)
    target_link_libraries(chxchx-editor-smoke PRIVATE
        Qt6::Core Qt6::Gui Qt6::Widgets Qt6::Core5Compat chxchx_scintilla chxchx_lexilla)
    add_test(NAME native-editor-smoke COMMAND chxchx-editor-smoke)
    set_tests_properties(native-editor-smoke PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

    add_executable(chxchx-pty-smoke tests/pty_smoke.cpp
        src/integrations/pty_session.cpp src/integrations/pty_session.hpp)
    target_link_libraries(chxchx-pty-smoke PRIVATE Qt6::Core)
    if(WIN32)
        target_compile_definitions(chxchx-pty-smoke PRIVATE WINVER=0x0A00 _WIN32_WINNT=0x0A00)
    elseif(UNIX AND NOT APPLE AND CHXCHX_UTIL_LIBRARY)
        target_link_libraries(chxchx-pty-smoke PRIVATE ${CHXCHX_UTIL_LIBRARY})
    endif()
    add_test(NAME native-pty-smoke COMMAND chxchx-pty-smoke)
endif()
