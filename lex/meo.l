%{
// Parser for Command scripts written for the trading platform
%}
%%

[\t ]+ /* ignore whitespace */ ;

%%

main(){
    yylex();
}
