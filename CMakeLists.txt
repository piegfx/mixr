cmake_minimum_required(VERSION 3.26)
project(mixr C)

set(CMAKE_C_STANDARD 99)

add_library(mixr
        include/MxMixer.h
        src/MxMixer.c)

target_include_directories(${PROJECT_NAME} PRIVATE include/)