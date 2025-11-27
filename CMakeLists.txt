cmake_minimum_required(VERSION 3.27)
set(MANIFOLD_EXPORT ON)
set(MANIFOLD_CBIND ON)
set(MANIFOLD_TEST OFF)
add_subdirectory(vendor/manifold PUBLIC)
# project(yascad LANGUAGES CXX)
# add_executable(yascad src/main.cpp)

# set(CMAKE_CXX_STANDARD 20)


# target_include_directories(yascad PUBLIC vendor/manifold/include)
# target_link_libraries(yascad PRIVATE manifold)
