cmake_minimum_required(VERSION 3.27)
project(yascad LANGUAGES CXX)
add_executable(yascad src/main.cpp)

set(CMAKE_CXX_STANDARD 20)

set(MANIFOLD_EXPORT ON)

add_subdirectory(vendor/manifold EXCLUDE_FROM_ALL)
target_include_directories(yascad PUBLIC vendor/manifold/include)
target_link_libraries(yascad PRIVATE manifold)
