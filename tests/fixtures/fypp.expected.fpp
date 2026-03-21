#:include 'macros.fpp'
module m_fypp_test
    implicit none
contains
    subroutine s_test()
        #:if defined('MFC_OpenACC')
            $:GPU_PARALLEL_LOOP(collapse=3)
            do k = 0, p
                do j = 0, n
                    do i = 0, m
                        x(i, j, k) = 0.0_wp
                    end do
                end do
            end do
            $:END_GPU_PARALLEL_LOOP()
        #:else
            do k = 0, p
                do j = 0, n
                    do i = 0, m
                        x(i, j, k) = 0.0_wp
                    end do
                end do
            end do
        #:endif
    end subroutine s_test

end module m_fypp_test
