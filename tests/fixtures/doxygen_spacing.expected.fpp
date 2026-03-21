module m_test
    implicit none
contains

    subroutine s_first()
        integer :: x
        x = 1
    end subroutine s_first

    !> @brief Second procedure Does something else
    subroutine s_second()
        integer :: y
        y = 2
    end subroutine s_second

    !> Third procedure with a long description that documents what this function does
    function f_third() result(z)
        integer :: z
        z = 3
    end function f_third

    !> Fourth procedure
    subroutine s_fourth()
        integer :: w
        w = 4
    end subroutine s_fourth
end module m_test
