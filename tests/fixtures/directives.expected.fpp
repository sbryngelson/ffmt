subroutine s_test()
    integer :: i, j, k
#if defined(MFC_OpenACC)
    !$acc parallel loop collapse(3)

    do k = 0, p
        do j = 0, n
            do i = 0, m
                x(i, j, k) = y(i, j, k)*z(i, j, k)
            end do
        end do
    end do
    !$acc end parallel loop
#endif

end subroutine s_test
