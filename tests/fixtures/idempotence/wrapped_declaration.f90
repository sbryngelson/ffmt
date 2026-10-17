module m
contains
    subroutine s()
        character(LEN = name_len) :: dir_name !<
        !! Name of the case directory
        character(LEN = name_len) :: file_name = 'run_time.inf' !<
        !! Name of the run-time information file
        x = 1
    end subroutine s
end module m
