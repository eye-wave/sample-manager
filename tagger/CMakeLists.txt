cmake_minimum_required(VERSION 3.10)
project(libtagger C)

add_library(libtagger STATIC src/libtagger.c)

set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)
target_compile_options(libtagger PRIVATE -Os)

target_link_options(libtagger PRIVATE
    -nostdlib
    -nostartfiles
    -Wl,--gc-sections
    -s
)
